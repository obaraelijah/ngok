mod packets;
use clap::Parser;

#[derive(Parser,Debug)]
struct Config {
    port: u16,
    domains: Vec<String>,
}


fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    if config.domains.is_empty() {
        panic!("DOmains is expected");
    }

    let mut port = config.port;
}


fn handle_connection() -> std::io::Result<()> {
    unimplemented!()
}