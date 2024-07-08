use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use log::{error, info};
use rand::distributions::WeightedError::AllWeightsZero;
use tokio::net::TcpListener;

use crate::conf::{Config, HttpConfig, HttpsConfig, TcpConfig};
use crate::connection_handle::{HttpsTunnel, HttpTunnel, serve, TcpTunnel};
use crate::tcp_connector::{ATcpConnector, TcpConnector};

mod handshake_codec;
mod conf;
mod tls_codec;
mod dns;
mod tcp_connector;
mod connection_handle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let conf = Config::from_cmd_line()?;

    let resolver = Arc::new(dns::DnsResolver::new());
    let tcp_connector = Arc::new(TcpConnector::new(conf.tunnel_config.client_connection.clone(), resolver.clone()));

    let mut join_handle_list = vec![];

    if let Some(ref http_conf) = conf.http {
        let jh = tokio::spawn({
            let http_conf = http_conf.clone();
            let tcp_connector = tcp_connector.clone();
            async move {
                serve_http_tunnel(http_conf, tcp_connector).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }
    if let Some(ref https_conf) = conf.https {
        let jh = tokio::spawn({
            let https_conf = https_conf.clone();
            let tcp_connector = tcp_connector.clone();
            async move {
                serve_https_tunnel(https_conf, tcp_connector).await?;
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
                serve_tcp_tunnel(tcp_conf, tcp_connector).await?;
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

pub async fn serve_http_tunnel(http_config: HttpConfig, tcp_connector: ATcpConnector) -> anyhow::Result<()> {
    let http_tunnel = HttpTunnel::new(http_config, tcp_connector);
    serve(Arc::new(http_tunnel)).await
}

pub async fn serve_https_tunnel(https_config: HttpsConfig, tcp_connector: ATcpConnector) -> anyhow::Result<()> {
    let https_tunnel = HttpsTunnel::new(https_config, tcp_connector);
    serve(Arc::new(https_tunnel)).await
}

pub async fn serve_tcp_tunnel(tcp_config: TcpConfig, tcp_connector: ATcpConnector) -> anyhow::Result<()> {
    let tcp_tunnel = TcpTunnel::new(tcp_config, tcp_connector);
    serve(Arc::new(tcp_tunnel)).await
}