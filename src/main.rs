mod config;
mod udp;
mod tcp;
mod pipe;

use tokio::{io::{AsyncWriteExt, ReadBuf}, net::TcpListener, time::timeout};
use std::{net::SocketAddr, pin::Pin};

trait PeekWraper {
    async fn peek(&self) -> tokio::io::Result<()>;
}
impl PeekWraper for tokio::net::TcpStream {
    async fn peek(&self) -> tokio::io::Result<()> {
        let mut buf = [0; 2];
        let mut wraper = ReadBuf::new(&mut buf);
        std::future::poll_fn(|cx| match self.poll_peek(cx, &mut wraper) {
            std::task::Poll::Pending => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(Ok(size)) => {
                if size == 0 {
                    std::task::Poll::Ready(Err(tokio::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "Peek: ConnectionAborted",
                    )))
                } else {
                    std::task::Poll::Ready(Ok(()))
                }
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
        })
        .await
    }
}

pub fn unsafe_staticref<'a, T: ?Sized>(r: &'a T) -> &'static T {
    unsafe { std::mem::transmute::<&'a T, &'static T>(r) }
}

#[tokio::main]
async fn main() {
    let conf = config::load_config();
    // Generate tasks for tcp
    for target in conf.tcp_proxy {
        // IPv4
        tokio::spawn(async move {
            let listen = SocketAddr::new(conf.listen_ip, target.port());
            println!("TCP listening on {} proxy to {}", listen, target);
            let tcp = TcpListener::bind(listen).await.unwrap();
            loop {
                if let Ok(stream) = tcp.accept().await {
                    let peer_addr = stream.1;
                    tokio::spawn(async move {
                        if let Err(e) = stream_handler(target, stream.0, conf.tcptimeout, conf.tcp_buffer_size.unwrap_or(8)).await {
                            if conf.log_error {
                                println!("TCP {peer_addr}: {e}");
                            }
                        }
                    });
                }
            }
        });
    }

    // Generate tasks for udp
    for target in conf.udp_proxy {
        tokio::spawn(async move {
            udp::udp_listen_handler(
                target.ip(),
                conf.udptimeout,
                SocketAddr::new(conf.listen_ip, target.port()),
            )
            .await;
        });
    }

    std::future::pending::<()>().await
}

async fn stream_handler(
    target: SocketAddr,
    stream: tokio::net::TcpStream,
    tm: u64,
    buf_size: usize
) -> Result<(), std::io::Error> {
    let target = tokio::net::TcpStream::connect(target).await?;

    let (ch_snd, mut ch_rcv) = tokio::sync::mpsc::channel(10);
    let stream_ghost = unsafe_staticref(&stream);
    let timeout_handler = async move {
        let mut dur = 0;
        loop {
            if dur >= tm {
                break;
            }
            // idle mode
            match timeout(
                std::time::Duration::from_secs(tm / 10),
                async { ch_rcv.recv().await },
            )
            .await
            {
                Err(_) => dur += tm / 10,
                Ok(None) => break,
                _ => {
                    dur = 0;
                    continue;
                }
            };
            // check if connection is alive using peek
            PeekWraper::peek(stream_ghost).await?;
        }

        Err::<(), tokio::io::Error>(tokio::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Connection idle timeout",
        ))
    };

    let (client_read, mut client_write) = tokio::io::split(stream);
    let (target_read, mut target_write) = tokio::io::split(target);

    let mut tcpwriter_client = tcp::TcpWriterGeneric {
        hr: Pin::new(&mut client_write),
        signal: ch_snd.clone(),
    };

    let mut tcpwriter_target = tcp::TcpWriterGeneric {
        hr: Pin::new(&mut target_write),
        signal: ch_snd,
    };
    
    if let Err(e) = tokio::try_join!(
        pipe::stack_copy(client_read, &mut tcpwriter_target, buf_size),
        pipe::stack_copy(target_read, &mut tcpwriter_client, buf_size),
        timeout_handler,
    ) {
        let _ = tcpwriter_target.shutdown().await;
        let _ = tcpwriter_client.shutdown().await;
        return Err(e);
    }
    Ok(())
}