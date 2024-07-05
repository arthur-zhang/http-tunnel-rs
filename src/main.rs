use std::sync::Arc;
use log::info;

use tokio::net::TcpListener;

use crate::conf::{Config, TcpConfig};
use crate::dns_resolver::SimpleCachingDnsResolver;

mod handshake_codec;
mod conf;
mod tunnel;
mod tls_codec;
mod dns_resolver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let conf = Config::from_cmd_line()?;
    let conf = Arc::new(conf);
    let dns_resolver = SimpleCachingDnsResolver::new(conf.tunnel_config.target_connection.dns_cache_ttl);

    let mut join_handle_list = vec![];

    if let Some(ref http_conf) = conf.http {
        let jh = tokio::spawn({
            let conf = conf.clone();
            let dns_resolver = dns_resolver.clone();
            let port = http_conf.listen_port;
            async move {
                serve_plain_text(conf, port, dns_resolver).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }
    if let Some(ref https_conf) = conf.https {
        let jh = tokio::spawn({
            let conf = conf.clone();
            let dns_resolver = dns_resolver.clone();
            let port = https_conf.listen_port;
            async move {
                serve_https(conf, port, dns_resolver).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }
    for tcp_conf in &conf.tcp {
        let jh = tokio::spawn({
            let conf = conf.clone();
            let tcp_conf = tcp_conf.clone();
            let dns_resolver = dns_resolver.clone();

            async move {
                serve_tcp(conf, dns_resolver, tcp_conf).await?;
                Ok::<(), anyhow::Error>(())
            }
        });
        join_handle_list.push(jh);
    }

    for jh in join_handle_list {
        let _ = jh.await?;
    }

    info!("Proxy stopped");
    Ok(())
}

async fn serve_tcp(config: Arc<Config>, dns_resolver: SimpleCachingDnsResolver, tcp_conf: TcpConfig) -> anyhow::Result<()> {
    let bind_address = format!("0.0.0.0:{}", tcp_conf.listen_port);
    let listener = TcpListener::bind(&bind_address).await.expect(&format!("tcp tunnel bind error {}", bind_address));

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.nodelay();
                tokio::spawn({
                    let dns_resolver = dns_resolver.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream);
                        let _ = tunnel.start_serv_http(dns_resolver).await;
                    }
                });
            }
            Err(e) => info!("Failed TCP handshake {}", e),
        }
    }

    Ok(())
}

async fn serve_plain_text(config: Arc<Config>, port: u16, dns_resolver: SimpleCachingDnsResolver) -> anyhow::Result<()> {
    let bind_address = format!("0.0.0.0:{}", port);
    info!("serving http requests on: {bind_address}");
    let listener = TcpListener::bind(&bind_address).await.expect(&format!("http tunnel bind error {}", bind_address));

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.nodelay();
                tokio::spawn({
                    let dns_resolver = dns_resolver.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream);
                        let _ = tunnel.start_serv_http(dns_resolver).await;
                    }
                });
            }
            Err(e) => info!("Failed TCP handshake {}", e),
        }
    }
}

async fn serve_https(config: Arc<Config>, port: u16, dns_resolver: SimpleCachingDnsResolver) -> anyhow::Result<()> {
    let bind_address = format!("0.0.0.0:{}", port);
    info!("serving https requests on: {bind_address}");
    let listener = TcpListener::bind(&bind_address).await.expect(&format!("http tunnel bind error {}", bind_address));

    loop {
        let socket = listener.accept().await;
        match socket {
            Ok((stream, _)) => {
                let _ = stream.nodelay();
                tokio::spawn({
                    let dns_resolver = dns_resolver.clone();
                    async move {
                        let mut tunnel = tunnel::TunnelConn::new(stream);
                        let _ = tunnel.start_serv_https(dns_resolver).await;
                    }
                });
            }
            Err(e) => info!("Failed TCP handshake {}", e),
        }
    }
}
