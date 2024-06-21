use std::io::BufRead;

use anyhow::bail;
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
pub struct DecodeResult<'a> {
    pub is_connect: bool,

    pub host: &'a str,
    pub port: u16,
    pub method: String,
    pub raw_header_bytes: Bytes,
    pub body: Option<Bytes>,
}

impl<'a> Decoder for HandshakeCodec {
    type Item = DecodeResult<'a>;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut headers = [httparse::EMPTY_HEADER; 3];
        let mut request = httparse::Request::new(&mut headers);

        return match request.parse(src)? {
            Status::Complete(n) => {
                let method = match request.method {
                    None => {
                        bail!("method not found")
                    }
                    Some(method) => {
                        method.to_string()
                    }
                };

                let is_connect = &method == "CONNECT";

                let mut host = "";
                let mut port = 0;


                let mut host_header_line = if is_connect {
                    request.path
                } else {
                    request.headers.iter().find_map(|header| {
                        if header.name.to_lowercase() == "host" {
                            std::str::from_utf8(header.value).ok()
                        } else {
                            None
                        }
                    })
                };

                if host_header_line.is_none() {
                    bail!("host not found")
                }
                let host_header_line = host_header_line.unwrap();

                let mut path_header = host_header_line.split(":");

                host = path_header.next().unwrap_or("").to_string();
                port = match path_header.next() {
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
                        port.parse::<u16>()?
                    }
                };
                let header_bytes = src.split_to(n).freeze();
                let maybe_body = src.split().freeze();

                Ok(Some(DecodeResult { is_connect, host: host.to_string(), port, method: method.to_string(), raw_header_bytes: header_bytes, body: Some(maybe_body) }))
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
    #[test]
    fn test_parse() {
        let mut headers = [httparse::EMPTY_HEADER; 64];

        // let req_data = r#"POST / HTTP/1.1\r\nContent-Type: application/json; charset=utf-8\r\nHost: localhost:8888\r\nContent-Length: 17\r\n\r\n{"name":"arthur"}"#;

        // let buf = b"GET /index.html HTTP/1.1\r\nHost: example.domain\r\n\r\n";

        let req_data = "POST / HTTP/1.1\r\nHost: localhost:8888\r\nContent-Length: 17\r\n\r\n123";
        println!("req_data: {:?}", req_data.len());
        let req = httparse::Request::new(&mut headers).parse(req_data.as_bytes());
        print!("req: {:?}", req);
    }
}