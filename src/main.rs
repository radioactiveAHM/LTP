mod config;
mod pipe;
mod udp;

use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::TcpListener};

#[tokio::main(flavor = "current_thread")]
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
                        if let Err(e) = stream_handler(
                            target,
                            stream.0,
                            conf.tcptimeout,
                            conf.tcp_buffer_size.unwrap_or(8),
                            conf.tcp_fill_buffer,
                        )
                        .await
                            && conf.log_error
                        {
                            println!("TCP {peer_addr}: {e}");
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
                conf.log_error,
                conf.udp_buffer_size.unwrap_or(8),
                conf.udp_channel_buffer_size,
            )
            .await;
        });
    }

    std::future::pending::<()>().await
}

async fn stream_handler(
    target: SocketAddr,
    mut stream: tokio::net::TcpStream,
    tm: u64,
    buf_size: usize,
    fill_buf: bool,
) -> Result<(), std::io::Error> {
    let mut target = tokio::net::TcpStream::connect(target).await?;

    let mut client_buf = vec![0; 1024 * buf_size];
    let mut client_buf_rb = tokio::io::ReadBuf::new(&mut client_buf);
    let mut target_buf = vec![0; 1024 * buf_size];
    let mut target_buf_rb = tokio::io::ReadBuf::new(&mut target_buf);

    let (mut client_r, mut client_w) = stream.split();
    let (mut target_r, mut target_w) = target.split();

    let mut client_r_pin = std::pin::Pin::new(&mut client_r);
    let mut target_r_pin = std::pin::Pin::new(&mut target_r);

    let err: tokio::io::Error;
    {
        loop {
            let operation = if fill_buf {
                tokio::time::timeout(std::time::Duration::from_secs(tm), async {
                    tokio::select! {
                        read = pipe::Fill(&mut client_r_pin, &mut client_buf_rb) => {
                            if !client_buf_rb.filled().is_empty() {
                                target_w.write_all(client_buf_rb.filled()).await?;
                            }
                            read
                        }
                        read = pipe::Fill(&mut target_r_pin, &mut target_buf_rb) => {
                            if !target_buf_rb.filled().is_empty() {
                                client_w.write_all(target_buf_rb.filled()).await?;
                            }
                            read
                        }
                    }
                })
                .await
            } else {
                tokio::time::timeout(std::time::Duration::from_secs(tm), async {
                    tokio::select! {
                        read = pipe::Read(&mut client_r_pin, &mut client_buf_rb) => {
                            read?;
                            target_w.write_all(client_buf_rb.filled()).await
                        }
                        read = pipe::Read(&mut target_r_pin, &mut target_buf_rb) => {
                            read?;
                            client_w.write_all(target_buf_rb.filled()).await
                        }
                    }
                })
                .await
            };

            match operation {
                Err(_) => {
                    err = tokio::io::Error::other("Timeout");
                    break;
                }
                Ok(Err(e)) => {
                    err = e;
                    break;
                }
                _ => (),
            }
        }
    }
    let _ = stream.shutdown().await;
    let _ = target.shutdown().await;
    Err(err)
}
