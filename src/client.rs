use bytes::{Buf, BytesMut};
use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::error::Error;

mod packet;

#[derive(Debug, Parser)]
struct Config {
    #[clap(short, long, value_parser)]
    domain_name: String,
    #[clap(short, long, value_parser, default_value_t = 3001)]
    server_port: u32,
    #[clap(short, long, value_parser, default_value_t = 4000)]
    target_port: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let config = Config::parse();
    tracing::trace!("Init client with {:?}", config);

    let domain = init(&config).await?;
    Ok(())
}

async fn init(config: &Config) -> Result<String, Box<dyn Error + Send + Sync>> {
    let cc = TcpStream::connect(format!("{}:{}", config.domain_name, config.server_port)).await?;
    let mut buf = BytesMut::with_capacity(1024);

    // Send a Init
    let init = packet::Packet::Init;
    cc.write_all(&bincode::serialize(&init).unwrap()).await?;
    let len = cc.read_buf(&mut buf).await?;
    let domain = if let packet::Packet::Success(domain) = packet::Packet::parse(&buf) {
        println!("Tunnel up!\nHost: {domain}");
        Some(domain)
    } else {
        None
    };
    buf.advance(len);

    if domain.is_none() {
        return Err("fail to init with server".into());
    }
    
    Ok(())
}
