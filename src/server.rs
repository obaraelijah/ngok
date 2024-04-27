mod packets;
use bytes::BytesMut;
use clap::Parser;

#[derive(Parser,Debug)]
struct Config {
    port: u16,
    domains: Vec<String>,
}


fn main() -> std::io::Result<()>{
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    if config.domains.is_empty() {
        panic!("DOmains is expected");
    }

    tracing::trace!("Init server with {:?}", config);

    let mut port = config.port;
    Ok(())
}


fn handle_connection() -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);
    Ok(())
}