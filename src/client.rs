use bytes::{Buf, BytesMut};
use clap::Parser;
use std::error::Error;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

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
    if let Err(e) = run_data_channel(&config, domain).await {
        tracing::error!("{:?}", e);
    }

    Ok(())
}

async fn init(config: &Config) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut cc =
        TcpStream::connect(format!("{}:{}", config.domain_name, config.server_port)).await?;
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

    // Let tunnel know client is ready
    cc.write_all(&bincode::serialize(&packet::Packet::Ack).unwrap())
        .await?;

    tracing::trace!("control channel established!");

    // send heartbeat to server every 500ms
    tokio::spawn(async move {
        loop {
            let res = cc.write_all(&[1u8; 1]).await;
            if let Err(err) = res {
                tracing::error!("control channel is closed by remote peer {}", err);
                break;
                let mut logger_dest = unimplemented!();
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    domain.ok_or_else(|| "no domain return".into())
}

async fn run_data_channel(config: &Config, domain: String) -> std::io::Result<()> {
    loop {
        let mut conn =
            TcpStream::connect(format!("{}:{}", config.domain_name, config.server_port)).await?;
        tracing::trace!("established data channel");
        conn.write_all(&bincode::serialize(&packet::Packet::DataInit(domain.clone())).unwrap())
            .await?;

        let packet = bincode::serialize(&packet::Packet::DataForward).unwrap();
        let mut buf = vec![0u8; packet.len()];
        conn.read_buf(&mut buf).await?;

        // Two implementations:
        // 2 -- > reimplement copy_bidirectional with an event channel that backdoors
        //          A --------> B           send(Event::AtoB).await.expect("channel closed!");
        //          B --------> A           j

        if let packet::Packet::DataForward = packet::Packet::parse(&buf) {
            let local = TcpStream::connect(format!("0.0.0.0:{}", config.target_port)).await?;
            tracing::trace!("copy bidirectional data: conn, local");

            let state = Arc::new(Mutex::new(LoggerState::new()));
            let mut logger_src = Logger {
                inner: Box::pin(conn),
                state: state.clone(),
            };

            let mut logger_dest = Logger {
                inner: Box::pin(local),
                state: state.clone(),
            };

            let _ = tokio::io::copy_bidirectional(&mut logger_src, &mut logger_dest);
        }
    }
}

struct LoggerState {
    timestamp: Option<Instant>,
}

struct Logger<T: AsyncRead + AsyncWrite> {
    // why do we need to box T? deref?
    inner: Pin<Box<T>>,
    state: Arc<Mutex<LoggerState>>,
}

impl LoggerState {
    fn new() -> Self {
        Self { timestamp: None }
    }
}

impl<T: AsyncRead + AsyncWrite> AsyncRead for Logger<T> {
    unimplemented!();
}

impl<T: AsyncWrite + AsyncRead> AsyncWrite for Logger<T> {
    unimplemented!();
}
