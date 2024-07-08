use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use rand::prelude::SliceRandom;
use rand::thread_rng;

use crate::conf::ClientConnectionConfig;
use crate::dns::TDNSResolver;

pub type ATcpConnector  = Arc<TcpConnector>;

pub struct TcpConnector {
    client_connection_config: ClientConnectionConfig,
    dns_resolver: TDNSResolver,
}


impl TcpConnector {
    pub fn new(client_connection_config: ClientConnectionConfig, dns_resolver: TDNSResolver) -> Self {
        Self { client_connection_config, dns_resolver }
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
        Ok(tokio::net::TcpStream::connect(sock_addr).await?)
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
        let dns_resolver = TDNSResolver::new(DnsResolver::new());
        let client_connection_config = ClientConnectionConfig::default();
        let tcp_connector = TcpConnector::new(client_connection_config, dns_resolver);
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

