use tokio::net::{TcpListener, TcpStream};

use crate::conf::Conf;

mod handshake_codec;
mod conf;
mod tunnel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let conf = Conf::from_cmd_line();
    let listener = TcpListener::bind(&conf.bind).await.expect(&format!("http tunnel bind error {}", conf.bind));
    println!("http tunnel server started at: {}", conf.bind);
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        println!("new connection: {:?}", peer_addr);
        tokio::spawn(async move {
            let mut tunnel = tunnel::TunnelConn::new(stream);
            let _ = tunnel.start_serv().await;
        });
    }

    Ok(())
}
