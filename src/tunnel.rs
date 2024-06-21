use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

use crate::handshake_codec::HandshakeCodec;

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
    pub async fn start_serv(mut self) {
        let stream = self.stream;
        let (r, mut w) = stream.into_split();
        let mut r = FramedRead::new(r, HandshakeCodec::new());

        let header_pkt = r.next().await.unwrap().unwrap();

        println!("header pkt: {:?}", header_pkt);

        let (mut remote_r, mut remote_w) =
            if header_pkt.is_connect {
                let addr = format!("{}:{}", header_pkt.host, header_pkt.port);
                let remote_conn = TcpStream::connect(addr).await.unwrap();
                w.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                w.flush().await.unwrap();
                remote_conn.into_split()
            } else {
                let addr = format!("{}:{}", header_pkt.host, header_pkt.port);
                let remote_conn = TcpStream::connect(addr).await.unwrap();

                let (mut remote_r, mut remote_w) = remote_conn.into_split();

                remote_w.write_all(&header_pkt.raw_header_bytes).await.unwrap();
                if let Some(body) = header_pkt.body {
                    remote_w.write_all(&body).await.unwrap();
                }
                (remote_r, remote_w)
            };


        let r = r.into_inner();
        let client_stream = r.reunite(w).unwrap();
        let (mut client_r, mut client_w) = client_stream.into_split();

        tokio::try_join!(
                tokio::io::copy(&mut remote_r, &mut client_w),
                tokio::io::copy(&mut client_r, &mut remote_w),
        );
    }
}
