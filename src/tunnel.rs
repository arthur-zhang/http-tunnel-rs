use std::sync::Arc;

use anyhow::bail;
use log::info;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

use crate::conf::TcpConfig;
use crate::handshake_codec::HandshakeCodec;
use crate::tcp_connector::TcpConnector;
use crate::tls_codec::TlsCodec;

pub struct TunnelConn {
    stream: TcpStream,
    tcp_connector: Arc<TcpConnector>,
}

impl TunnelConn {
    pub fn new(stream: TcpStream, tcp_connector: Arc<TcpConnector>) -> Self {
        Self {
            stream,
            tcp_connector,
        }
    }
}


impl TunnelConn {
    pub async fn start_serv_http(mut self) -> anyhow::Result<()> {
        let stream = self.stream;
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
    pub async fn start_serv_https(mut self) -> anyhow::Result<()> {
        let stream = self.stream;
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

    pub async fn start_serv_tcp(mut self, tcp_config: TcpConfig) -> anyhow::Result<()> {
        let remote_addr = tcp_config.remote_addr;
        use anyhow::{bail, Result};

        let (host, port) = match remote_addr.rsplit_once(':') {
            Some((host, port)) => {
                let host = host.to_owned();
                let port = port.parse::<u16>()?;
                (host, port)
            }
            None => bail!("invalid remote addr: {}", remote_addr),
        };

        let mut remote_conn = self.tcp_connector.connect(&host, port).await?;
        tokio::io::copy_bidirectional(&mut self.stream, &mut remote_conn).await?;
        Ok(())
    }
}
