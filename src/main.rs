use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use log::{error, info};
use tokio::net::TcpListener;

use crate::conf::{Config, TcpConfig};
use crate::tcp_connector::TcpConnector;

mod handshake_codec;
mod conf;
mod tunnel;
mod tls_codec;
mod dns;
mod tcp_connector;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let conf = Config::from_cmd_line()?;

    let resolver = Arc::new(dns::DnsResolver::new());
    let tcp_connector = Arc::new(TcpConnector::new(conf.tunnel_config.client_connection.clone(), resolver.clone()));

    let mut join_handle_list = vec![];

    if let Some(ref http_conf) = conf.http {
        let jh = tokio::spawn({
            let port = http_conf.listen_port;
            let tcp_connector = tcp_connector.clone();
            async move {
                serve_http(port, tcp_connector).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }
    if let Some(ref https_conf) = conf.https {
        let jh = tokio::spawn({
            let tcp_connector = tcp_connector.clone();
            let port = https_conf.listen_port;
            async move {
                serve_https(port, tcp_connector).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }
    for tcp_conf in &conf.tcp {
        let jh = tokio::spawn({
            let tcp_conf = tcp_conf.clone();
            let tcp_connector = tcp_connector.clone();
            async move {
                serve_tcp(tcp_conf, tcp_connector.clone()).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }

    for jh in join_handle_list {
        let join_result = jh.await;
        if let Err(e) = join_result {
            error!("join error: {}", e);
        }
    }

    error!("proxy stopped");
    Ok(())
}

async fn serve_tcp(tcp_conf: TcpConfig, tcp_connector: Arc<TcpConnector>) -> anyhow::Result<()> {
    let bind_address = format!("0.0.0.0:{}", tcp_conf.listen_port);
    info!("serving tcp requests on: {bind_address}");
    let listener = TcpListener::bind(&bind_address).await?;

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.set_nodelay(true);
                tokio::spawn({
                    let tcp_conf = tcp_conf.clone();
                    let tcp_connector = tcp_connector.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream, tcp_connector);
                        let _ = tunnel.start_serv_tcp(tcp_conf).await;
                    }
                });
            }
            Err(e) => info!("Failed TCP handshake {}", e),
        }
    }

    Ok(())
}

async fn serve_http(listen_port: u16, tcp_connector: Arc<TcpConnector>) -> anyhow::Result<()> {
    let listen_sock_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, listen_port);
    info!("serving http requests on: {listen_sock_addr}");

    let listener = TcpListener::bind(&listen_sock_addr).await?;

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.set_nodelay(true);
                tokio::spawn({
                    let tcp_connector = tcp_connector.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream, tcp_connector);
                        let _ = tunnel.start_serv_http().await;
                    }
                });
            }
            Err(e) => info!("Failed TCP handshake {}", e),
        }
    }
}

async fn serve_https(port: u16, tcp_connector: Arc<TcpConnector>) -> anyhow::Result<()> {
    let bind_address = format!("0.0.0.0:{}", port);
    info!("serving https requests on: {bind_address}");
    let listener = TcpListener::bind(&bind_address).await?;

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.set_nodelay(true);
                tokio::spawn({
                    let tcp_connector = tcp_connector.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream, tcp_connector);
                        let _ = tunnel.start_serv_https().await;
                    }
                });
            }
            Err(e) => info!("https accept failed: {}", e),
        }
    }
}
