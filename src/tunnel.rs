use log::info;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

use crate::conf::TcpConfig;
use crate::dns_resolver::SimpleCachingDnsResolver;
use crate::handshake_codec::HandshakeCodec;
use crate::tls_codec::TlsCodec;

pub struct TunnelConn {
    stream: TcpStream,
}

impl TunnelConn {
    pub fn new(stream: TcpStream) -> Self {
        stream.nodelay().unwrap();
        Self {
            stream
        }
    }
}


impl TunnelConn {
    pub async fn start_serv_http(mut self, mut dns_resolver: SimpleCachingDnsResolver) -> anyhow::Result<()> {
        let stream = self.stream;
        let (r, mut w) = stream.into_split();
        let mut r = FramedRead::new(r, HandshakeCodec::new());

        let header_pkt = r.next().await.ok_or(anyhow::anyhow!("no header pkt"))??;
        info!("header pkt: {:?}", header_pkt);

        let addr = format!("{}:{}", header_pkt.host, header_pkt.port);

        info!("start resolve: {}", addr);
        let remote_addr = dns_resolver.resolve(&addr).await?;
        info!("resolved: {}->{}", addr, remote_addr);

        let mut remote_conn = TcpStream::connect(remote_addr).await?;

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
    pub async fn start_serv_https(mut self, mut dns_resolver: SimpleCachingDnsResolver) -> anyhow::Result<()> {
        let stream = self.stream;
        let (r, mut w) = stream.into_split();
        let mut r = FramedRead::new(r, TlsCodec::new());

        let (sni, bytes) = r.next().await.ok_or(anyhow::anyhow!("no header pkt"))??;
        if sni.is_empty() {
            return Err(anyhow::anyhow!("no sni"));
        }
        let addr = format!("{}:443", sni);
        info!("start resolve https: {}", addr);
        let remote_addr = dns_resolver.resolve(&addr).await?;
        info!("resolved https: {}->{}", addr, remote_addr);

        let mut remote_conn = TcpStream::connect(&remote_addr).await?;

        remote_conn.write_all(&bytes).await?;
        remote_conn.flush().await?;

        let r = r.into_inner();
        let mut client_stream = r.reunite(w)?;
        tokio::io::copy_bidirectional(&mut client_stream, &mut remote_conn).await?;
        Ok(())
    }

    pub async fn start_serv_tcp(mut self, tcp_config: TcpConfig, mut dns_resolver: SimpleCachingDnsResolver) -> anyhow::Result<()> {
        info!("start resolve tcp: {}", tcp_config.remote_addr);
        let remote_addr = dns_resolver.resolve(&tcp_config.remote_addr).await?;
        info!("end resolved tcp: {}->{}", tcp_config.remote_addr, remote_addr);

        let mut remote_conn = TcpStream::connect(remote_addr).await?;

        tokio::io::copy_bidirectional(&mut self.stream, &mut remote_conn).await?;
        Ok(())
    }
}
