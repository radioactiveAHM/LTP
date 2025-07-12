mod config;
mod udp;
mod tcp;
mod pipe;

use tokio::{io::AsyncWriteExt, net::TcpListener};
use std::{net::SocketAddr, pin::Pin};


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
    mut stream: tokio::net::TcpStream,
    tm: u64,
    buf_size: usize
) -> Result<(), std::io::Error> {
    let mut target = tokio::net::TcpStream::connect(target).await?;

    let (client_read, mut client_write) = stream.split();
    let (target_read, mut target_write) = target.split();

    let mut tcpwriter_client = tcp::TcpWriterGeneric {
        hr: Pin::new(&mut client_write)
    };

    let mut tcpwriter_target = tcp::TcpWriterGeneric {
        hr: Pin::new(&mut target_write)
    };
    
    if let Err(e) = tokio::try_join!(
        pipe::copy(client_read, &mut tcpwriter_target, buf_size, tm),
        pipe::copy(target_read, &mut tcpwriter_client, buf_size, tm)
    ) {
        let _ = tcpwriter_target.shutdown().await;
        let _ = tcpwriter_client.shutdown().await;
        return Err(e);
    }
    Ok(())
}