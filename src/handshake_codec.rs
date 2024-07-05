use std::io::BufRead;

use anyhow::{anyhow, bail};
use httparse::Status;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub struct HandshakeCodec {}

impl HandshakeCodec {
    pub fn new() -> Self {
        Self {}
    }
}

pub const INIT_HEADER_BUF_SIZE: usize = 4096;
pub const MAX_HEADER_SIZE: usize = 1048575;

#[derive(Debug)]
pub struct DecodeResult {
    pub is_connect: bool,
    pub host: String,
    pub port: u16,
    pub method: String,
    pub header_len: usize,
    pub req_body_bytes: Bytes,
}

fn extract_host_and_port(line: &str) -> anyhow::Result<(String, u16)> {
    let mut path_header = line.split(":");
    let mut host = path_header.next().unwrap_or("");
    let mut port = match path_header.next() {
        None => {
            if host.starts_with("http") {
                80
            } else if host.starts_with("https") {
                443
            } else {
                80
            }
        }
        Some(port) => {
            port.parse::<u16>().map_err(|_| anyhow!("port parse error"))?
        }
    };
    Ok((host.to_string(), port))
}

impl Decoder for HandshakeCodec {
    type Item = DecodeResult;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut headers = [httparse::EMPTY_HEADER; 256];
        let mut request = httparse::Request::new(&mut headers);

        return match request.parse(src)? {
            Status::Complete(n) => {
                let method = request.method.ok_or(anyhow::anyhow!("method not found"))?.to_string();

                let is_connect = method.eq_ignore_ascii_case("CONNECT");

                let mut host_header_line =
                    request.headers.iter().find_map(|header| {
                        if header.name.to_lowercase() == "host" {
                            std::str::from_utf8(header.value).ok()
                        } else {
                            None
                        }
                    });

                let host_header_line = host_header_line.ok_or(anyhow::anyhow!("host not found"))?;

                let (host, port) = extract_host_and_port(host_header_line)?;

                let req_body_bytes = src.split().freeze();

                Ok(Some(DecodeResult { is_connect, host, port, method, header_len: n, req_body_bytes }))
            }
            Status::Partial => {
                if src.len() >= MAX_HEADER_SIZE {
                    bail!("header too large")
                }
                Ok(None)
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use log::info;

    #[test]
    fn test_parse() {
        let mut headers = [httparse::EMPTY_HEADER; 64];

        // let req_data = r#"POST / HTTP/1.1\r\nContent-Type: application/json; charset=utf-8\r\nHost: localhost:8888\r\nContent-Length: 17\r\n\r\n{"name":"arthur"}"#;

        // let buf = b"GET /index.html HTTP/1.1\r\nHost: example.domain\r\n\r\n";

        let req_data = "POST / HTTP/1.1\r\nHost: localhost:8888\r\nContent-Length: 17\r\n\r\n123";
        info!("req_data: {:?}", req_data.len());
        let req = httparse::Request::new(&mut headers).parse(req_data.as_bytes());
        info!("req: {:?}", req);
    }

    #[test]
    fn test_parse_host_port() {
        let str = "CONNECT edulyse.test.sewo.com:80 HTTP/1.1";
    }
}