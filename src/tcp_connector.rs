use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use log::error;
use rand::prelude::SliceRandom;
use rand::thread_rng;

use crate::conf::TargetConnectionConfig;
use crate::dns;
use crate::dns::TDNSResolver;

pub type ATcpConnector = Arc<TcpConnector>;

pub struct TcpConnector {
    target_connection_config: TargetConnectionConfig,
    dns_resolver: TDNSResolver,
}

impl TcpConnector {
    pub fn new(target_connection_config: TargetConnectionConfig) -> Self {
        let dns_resolver = dns::DnsResolver::new(target_connection_config.dns_cache_ttl);
        let dns_resolver = Arc::new(dns_resolver);
        Self { target_connection_config, dns_resolver }
    }
    async fn to_socket_addr(&self, host: &str, port: u16) -> anyhow::Result<Vec<SocketAddr>> {
        if let Ok(addr) = host.parse::<Ipv4Addr>() {
            let addr = SocketAddrV4::new(addr, port);
            return Ok(vec![SocketAddr::V4(addr)]);
        }
        let addrs = self.dns_resolver.resolve(host).await?;
        Ok(addrs.iter().map(|it| SocketAddr::new(IpAddr::V4(*it), port)).collect())
    }

    pub async fn connect(&self, host: &str, port: u16) -> anyhow::Result<tokio::net::TcpStream> {
        let mut sock_addrs = self.to_socket_addr(host, port).await?;
        let sock_addr = sock_addrs.choose(&mut thread_rng()).ok_or(anyhow::anyhow!("No address found for host: {}", host))?;
        let connect_result = tokio::time::timeout(
            self.target_connection_config.connect_timeout,
            tokio::net::TcpStream::connect(sock_addr),
        ).await;
        let tcp_stream = match connect_result {
            Ok(Ok(tcp_stream)) => tcp_stream,

            Ok(Err(err)) => {
                error!("failed to connect to {}:{}, err: {:?}", host, port, err);
                return Err(err.into());
            }
            Err(elapsed) => {
                error!("connect timeout {}:{}, reach limit: {:?}", host, port, self.target_connection_config.connect_timeout);
                return Err(elapsed.into());
            }
        };

        let _ = tcp_stream.set_nodelay(true);
        Ok(tcp_stream)
    }
}

#[cfg(test)]
mod tests {
    use log::{debug, error, info};

    use crate::dns::DnsResolver;

    use super::*;

    #[tokio::test]
    async fn test_to_socket_addr() -> anyhow::Result<()> {
        env_logger::init();
        let dns_resolver = TDNSResolver::new(DnsResolver::new(None));
        let target_connection_config = TargetConnectionConfig::default();
        let tcp_connector = TcpConnector::new(target_connection_config);
        debug!("{:?}", "start");
        let addrs = tcp_connector.to_socket_addr("www.baidu.com", 80).await?;
        debug!("{:?}", addrs);
        let addrs = tcp_connector.to_socket_addr("192.168.31.8", 8080).await?;
        debug!("{:?}", addrs);
        let tcp_stream = tcp_connector.connect("www.baidu.com", 80).await?;
        debug!("tcp_stream: {:?}", tcp_stream);
        Ok(())
    }
}

