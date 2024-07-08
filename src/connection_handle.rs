use std::net::{Ipv4Addr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::bail;
use log::info;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

use crate::conf::{HttpConfig, HttpsConfig, TcpConfig};
use crate::handshake_codec::HandshakeCodec;
use crate::tcp_connector::ATcpConnector;
use crate::tls_codec::TlsCodec;

#[async_trait::async_trait]
pub trait TunnelHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn listen_addr(&self) -> (Ipv4Addr, u16);
    async fn handle_conn(&self, stream: TcpStream) -> anyhow::Result<()>;
}


pub struct HttpTunnel {
    http_config: HttpConfig,
    tcp_connector: ATcpConnector,
}

pub struct HttpsTunnel {
    https_config: HttpsConfig,
    tcp_connector: ATcpConnector,
}

impl HttpTunnel {
    pub fn new(http_config: HttpConfig, tcp_connector: ATcpConnector) -> Self {
        Self { http_config, tcp_connector }
    }
}

impl HttpsTunnel {
    pub fn new(https_config: HttpsConfig, tcp_connector: ATcpConnector) -> Self {
        Self { https_config, tcp_connector }
    }
}

pub struct TcpTunnel {
    tcp_config: TcpConfig,
    tcp_connector: ATcpConnector,
}

impl TcpTunnel {
    pub fn new(tcp_config: TcpConfig, tcp_connector: ATcpConnector) -> Self {
        Self { tcp_config, tcp_connector }
    }
}


pub async fn serve<T>(handler: Arc<T>) -> anyhow::Result<()>
    where T: TunnelHandler + 'static {
    let bind_addr = handler.listen_addr();
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("[{}] listening on: {:?}", handler.name(), bind_addr);
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn({
            let handler = handler.clone();
            async move {
                handler.handle_conn(stream).await
            }
        });
    }
    Ok(())
}

#[async_trait::async_trait]
impl TunnelHandler for HttpTunnel {
    fn name(&self) -> &'static str {
        "http_tunnel"
    }

    fn listen_addr(&self) -> (Ipv4Addr, u16) {
        (Ipv4Addr::UNSPECIFIED, self.http_config.listen_port)
    }

    async fn handle_conn(&self, stream: TcpStream) -> anyhow::Result<()> {
        let (r, mut w) = stream.into_split();
        let mut r = FramedRead::new(r, HandshakeCodec::new());
        let header_pkt = r.next().await.ok_or(anyhow::anyhow!("no header pkt"))??;
        info!("header pkt: {:?}", header_pkt);
        let mut remote_conn = self.tcp_connector.connect(&header_pkt.host, header_pkt.port).await?;

        if header_pkt.is_connect {
            w.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
            w.flush().await?;
        } else {
            remote_conn.write_all(&header_pkt.req_body_bytes).await?;
            remote_conn.flush().await?;
        };

        let r = r.into_inner();
        let mut client_stream = r.reunite(w)?;
        tokio::io::copy_bidirectional(&mut client_stream, &mut remote_conn).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl TunnelHandler for HttpsTunnel {
    fn name(&self) -> &'static str {
        "https_tunnel"
    }

    fn listen_addr(&self) -> (Ipv4Addr, u16) {
        (Ipv4Addr::UNSPECIFIED, self.https_config.listen_port)
    }

    async fn handle_conn(&self, stream: TcpStream) -> anyhow::Result<()> {
        let (r, mut w) = stream.into_split();
        let mut r = FramedRead::new(r, TlsCodec::new());

        let (sni, bytes) = r.next().await.ok_or(anyhow::anyhow!("no header pkt"))??;
        if sni.is_empty() {
            return Err(anyhow::anyhow!("no sni"));
        }
        let mut remote_conn = self.tcp_connector.connect(&sni, 443).await?;

        remote_conn.write_all(&bytes).await?;
        remote_conn.flush().await?;

        let r = r.into_inner();
        let mut client_stream = r.reunite(w)?;
        tokio::io::copy_bidirectional(&mut client_stream, &mut remote_conn).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl TunnelHandler for TcpTunnel {
    fn name(&self) -> &'static str {
        "tcp_tunnel"
    }

    fn listen_addr(&self) -> (Ipv4Addr, u16) {
        (Ipv4Addr::UNSPECIFIED, self.tcp_config.listen_port)
    }

    async fn handle_conn(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let remote_addr = &self.tcp_config.remote_addr;

        let (host, port) = match remote_addr.rsplit_once(':') {
            Some((host, port)) => {
                let host = host.to_owned();
                let port = port.parse::<u16>()?;
                (host, port)
            }
            None => bail!("invalid remote addr: {}", remote_addr),
        };

        let mut remote_conn = self.tcp_connector.connect(&host, port).await?;
        tokio::io::copy_bidirectional(&mut stream, &mut remote_conn).await?;
        Ok(())
    }
}

