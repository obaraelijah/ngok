mod packet;
use bytes::BytesMut;
use clap::Parser;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Config {
    #[clap(short, long, value_parser, default_value_t = 3001)]
    port: u16,
    #[clap(short, long, value_parser)]
    domains: Vec<String>,
}


#[tokio::main]
async fn main() -> std::io::Result<()>{
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    if config.domains.is_empty() {
        panic!("DOmains is expected");
    }

    tracing::trace!("Init server with {:?}", config);

    let mut port = config.port;
    Ok(())
}


fn handle_connection(
    mut conn: TcpStream
) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);
    let bytes_len = conn.read_buf(&mut buffer);
    let packet = packet::Packet::parse(&buffer);
    match packet {
        packet::Packet::Init => {
            unimplemented!()
        }
        packet::Packet::DataInit(domain) => {
            unimplemented!()
        }
    }
    Ok(())
}