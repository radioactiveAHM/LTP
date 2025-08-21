mod config;
mod pipe;
mod udp;

use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::TcpListener};

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
                        if let Err(e) = stream_handler(
                            target,
                            stream.0,
                            conf.tcptimeout,
                            conf.tcp_buffer_size.unwrap_or(8),
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
) -> Result<(), std::io::Error> {
    let mut target = tokio::net::TcpStream::connect(target).await?;

    let (client_read, mut client_write) = stream.split();
    let (target_read, mut target_write) = target.split();

    if let Err(e) = tokio::try_join!(
        pipe::copy(client_read, &mut target_write, buf_size, tm),
        pipe::copy(target_read, &mut client_write, buf_size, tm)
    ) {
        let _ = client_write.shutdown().await;
        let _ = target_write.shutdown().await;
        return Err(e);
    }
    Ok(())
}
