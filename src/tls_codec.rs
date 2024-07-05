use anyhow::bail;
use rustls::{ContentType, ProtocolVersion};
use rustls::internal::msgs::codec::{Codec, Reader};
use rustls::internal::msgs::handshake::{ClientExtension, HandshakeMessagePayload, HandshakePayload};
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub struct TlsCodec {}

impl TlsCodec {
    pub fn new() -> Self {
        Self {}
    }
}

pub type Sni = String;

impl Decoder for TlsCodec {
    type Item = (Sni, Bytes);
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }
        if src.len() < 5 {
            return Ok(None);
        }

        let mut reader = Reader::init(src.as_ref());
        let content_type = ContentType::read(&mut reader).map_err(|_| anyhow::anyhow!("content type error"))?;
        if content_type != ContentType::Handshake {
            bail!("not handshake");
        }
        let _version = ProtocolVersion::read(&mut reader).map_err(|_| anyhow::anyhow!("version error"))?;
        let len = u16::read(&mut reader).map_err(|_| anyhow::anyhow!("len error"))?;
        if src.len() < 5 + len as usize {
            return Ok(None);
        }
        let handshake_payload = HandshakeMessagePayload::read(&mut reader).map_err(|_| anyhow::anyhow!("handshake payload error"))?;

        if let HandshakePayload::ClientHello(client_hello) = handshake_payload.payload {
            let server_name =
                client_hello.extensions
                    .iter()
                    .find_map(|ext| match ext {
                        ClientExtension::ServerName(server_name_vec) => {
                            server_name_vec.first()
                        }
                        _ => None
                    })
                    .map(|it| {
                        let encoded = it.get_encoding();

                        if encoded.len() < 3 {
                            return Err(anyhow::anyhow!("server name too short"));
                        }
                        std::str::from_utf8(&encoded[3..]).map(|it| it.to_string()).map_err(|_| anyhow::anyhow!("server name not utf8"))
                    });

            match server_name {
                Some(Ok(server_name)) => {
                    let raw_data = src.split().freeze();
                    return Ok(Some((server_name, raw_data)));
                }
                _ => {
                    bail!("server name not found");
                }
            }
        }
        bail!("server name not found");
    }
}
