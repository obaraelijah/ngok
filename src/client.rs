use ansi_term::Colour;
use bytes::{Buf, BytesMut};
use clap::Parser;
use std::error::Error;
use std::pin::Pin;
use std::str;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt as _, ReadBuf};
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

            let _ = tokio::io::copy_bidirectional(&mut logger_src, &mut logger_dest).await;
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

/// "implement the AsyncRead trait for the Logger struct where T is bound to AsyncRead"
impl<T: AsyncRead + AsyncWrite> AsyncRead for Logger<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>, // hint: that this method is supposed to be ran inside of a task buf: &mut ReadBuf<'_>,
        buf: &mut ReadBuf<'_>, // data stored
    ) -> Poll<std::io::Result<()>> {
        let poll_result = self.inner.as_mut().poll_read(cx, buf);
        if poll_result.is_ready() {
            // do not log if the buffer is empty
            if buf.capacity() != buf.remaining() {
                // TODO: I think we can further optimized this by just
                // checking the first few bytes before constructing a str
                // from the buffer.
                //
                // Since, we are only interested in the first line of a HTTP payload,
                // we can just check if it's start with GET/POST/etc, HTTP/1.1 or HTTP/2
                if let Ok(raw_http) = str::from_utf8(buf.filled()) {
                    // NOTE: We are only interested in the very first line of the
                    // HTTP payload, which give us the HTTP method, path and status
                    // code.
                    //
                    // So, instead of collecting all of the splitted string
                    // into Vec, we just lazily iterate to the next one instead.
                    // In theory, this should reduce the memory usage since we
                    // are only consuming one line.
                    if let Some(line) = raw_http.split('\n').next() {
                        if !line.contains("HTTP/1.1") {
                            return poll_result;
                        };

                        let log = line.replace("HTTP/1.1", "");
                        let log = log.trim();

                        let mut state = self.state.lock().unwrap();
                        if let Some(instant) = state.timestamp.take() {
                            let Some((status_code, _status)) = log.split_once(' ') else {
                                return poll_result;
                            };

                            let Ok(status_code) = status_code.parse() else {
                                return poll_result;
                            };

                            let color_status = match status_code {
                                404 => Colour::Yellow.paint(log).to_string(),
                                status_code if status_code >= 400 => {
                                    Colour::Red.paint(log).to_string()
                                }
                                status_code if status_code >= 200 => {
                                    Colour::Green.paint(log).to_string()
                                }
                                _ => status_code.to_string(),
                            };

                            println!("{:<#15?} {color_status}", instant.elapsed());
                        } else {
                            print!("{:<20} ", log.trim());
                            state.timestamp = Some(Instant::now());
                        }
                    }

                    // let chunks: Vec<&str> = raw_http.split('\n').collect();
                    // if !chunks.is_empty() && chunks[0].contains("HTTP/1.1") {
                    //     let log = chunks[0].replace("HTTP/1.1", "");
                    //     let log = log.trim();
                    //
                    //     let mut state = self.state.lock().unwrap();
                    //     if let Some(instant) = state.timestamp.take() {
                    //         let Some((status_code, _status)) = log.split_once(' ') else {
                    //             return poll_result;
                    //         };
                    //
                    //         let Ok(status_code) = status_code.parse() else  {
                    //             return poll_result;
                    //         };
                    //
                    //         let color_status = match status_code {
                    //             404 => Colour::Yellow.paint(log).to_string(),
                    //             status_code if status_code >= 400 => {
                    //                 Colour::Red.paint(log).to_string()
                    //             }
                    //             status_code if status_code >= 200 => {
                    //                 Colour::Green.paint(log).to_string()
                    //             }
                    //             _ => status_code.to_string(),
                    //         };
                    //
                    //         println!("{:<#15?} {color_status}", instant.elapsed());
                    //     } else {
                    //         print!("{:<20} ", log.trim());
                    //         state.timestamp = Some(Instant::now());
                    //     }
                    //
                    // Unlock explicitly
                    // drop(state);
                    // }
                }
            }
        }
        poll_result
    }
}

impl<T: AsyncWrite + AsyncRead> AsyncWrite for Logger<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.inner.as_mut().poll_write(cx, buf)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        self.inner.as_mut().poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_shutdown(cx)
    }
}
