mod packet;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bytes::BytesMut;
use clap::Parser;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

const DEFAULT_IP: &str = "0.0.0.0";

type State = Arc<RwLock<HashMap<String, ControlChannel>>>;
type DomainPort = (String, u16);
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Config {
    #[clap(short, long, value_parser, default_value_t = 3001)]
    port: u16,
    #[clap(short, long, value_parser)]
    domains: Vec<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    if config.domains.is_empty() {
        panic!("domains is expected!");
    }

    tracing::trace!("init server with {:?}", config);

    let mut port = config.port;
    let domain_port_mapping: Vec<DomainPort> = config
        .domains
        .iter()
        .map(|domain| {
            port += 1;
            (domain.to_string(), port)
        })
        .collect();

    let listener = TcpListener::bind((DEFAULT_IP, config.port)).await?;
    let domain_to_port = Arc::new(Mutex::new(domain_port_mapping));
    tracing::trace!("domain mapping: {:?}", domain_to_port);

    let state: State = Arc::new(RwLock::new(HashMap::new()));
    tracing::info!("Listening on TCP: {DEFAULT_IP}:{}", config.port);
    loop {
        if let Ok((conn, _)) = listener.accept().await {
            tracing::info!("Accepting new client...");
            let state = state.clone();
            let domains = domain_to_port.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_connection(conn, domains, state).await {
                    tracing::error!("handle_connection: {err:?}");
                }
            });
        }
    }
}

async fn handle_connection(
    mut conn: TcpStream,
    domains: Arc<Mutex<Vec<DomainPort>>>,
    state: State,
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

struct  ControlChannel {
    data_tx: mpsc::Sender<TcpStream>,
}

impl ControlChannel {
    pub fn new(
        mut conn: TcpStream,
        domain_port: DomainPort,
        domains: Arc<Mutex<Vec<DomainPort>>>,
    ) -> Self {
        let (tx, mut rx): (_, mpsc::Receiver<TcpStream>) = mpsc::channel(32);
        let (close_tx, mut close_rx) = tokio::sync::oneshot::channel();

        // Push domain back to domain pools when client connection closed
        // The client is expected to a Heartbeat every 500ms
        let domains = Arc::clone(&domains);
        let dp = domain_port.clone();
        tokio::spawn(async move {
            loop {
                let mut buf = vec![0u8; 1];
                let res = conn.read_exact(&mut buf).await;
                if let Err(err) = res {
                    tracing::error!("receive error: {err}");
                    let mut domains_guard = domains.lock().await;
                    let domain_port = dp.clone();
                    domains_guard.push(domain_port);

                    let port = dp.1;
                    let _ = close_tx.send(port);
                    break;
                }
            }
        });

        ControlChannel { data_tx: tx }
    }
}
