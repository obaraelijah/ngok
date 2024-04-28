use clap::Parser;
use std::error::Error;

#[derive(Debug, Parser)]
struct Config {
    domain_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let config = Config::parse();
    tracing::trace!("Init client with {:?}", config);
    Ok(())
}

async fn init() -> Result<(), Box<dyn Error>> {
    unimplemented!();
    Ok(())
}
