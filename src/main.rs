mod config;
mod udp;
mod tcp;
mod pipe;

use tokio::{io::{AsyncWriteExt, ReadBuf}, net::TcpListener, time::timeout};
use std::{net::{IpAddr, SocketAddr}, pin::Pin};

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
    let ports: Vec<u16> = conf.ports;

    let mut tasks = Vec::new();
    // Generate tasks for tcp
    for port in ports {
        // IPv4
        tasks.push(tokio::spawn(async move {
            listen_handler(
                port,
                conf.target,
                SocketAddr::new(conf.listen_ip, port),
                conf.tcptimeout,
                conf.tcp_buffer_size.unwrap_or(8)
            )
            .await;
        }));
    }

    // Generate tasks for udp
    let ports: Vec<u16> = conf.udp_ports;
    for port in ports {
        // IPv4
        tasks.push(tokio::spawn(async move {
            udp::udp_listen_handler(
                conf.target,
                conf.udptimeout,
                SocketAddr::new(conf.listen_ip, port),
            )
            .await;
        }));
    }

    for atask in tasks {
        if let Err(e) = atask.await {
            println!("Task Error: {}", e);
        }
    }
}

async fn listen_handler(port: u16, target: IpAddr, inbound: SocketAddr, tm: u64, buf_size: usize) {
    let tcp = TcpListener::bind(inbound).await.unwrap();

    loop {
        if let Ok(stream) = tcp.accept().await {
            let peer_addr = stream.1;
            tokio::spawn(async move {
                if let Err(e) = stream_handler(port, target, stream.0, tm, buf_size).await {
                    println!("TCP {peer_addr}: {e}");
                }
            });
        }
    }
}

async fn stream_handler(
    port: u16,
    target: IpAddr,
    stream: tokio::net::TcpStream,
    tm: u64,
    buf_size: usize
) -> Result<(), std::io::Error> {
    let target = tokio::net::TcpStream::connect(SocketAddr::new(target, port)).await?;

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