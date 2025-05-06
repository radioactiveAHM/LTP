mod config;
mod udp;
mod tcp;

use tokio::{net::TcpListener, time::timeout};
use std::net::{IpAddr, SocketAddr};

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
                conf.tcp_buffer_size.unwrap_or(1024*8)
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

    // accept streams
    loop {
        if let Ok(stream) = tcp.accept().await {
            tokio::spawn(async move {
                if let Err(e) = stream_handler(port, target, stream.0, tm, buf_size).await {
                    println!("{e}");
                }
            });
        }
    }
}

async fn stream_handler(
    port: u16,
    target: IpAddr,
    mut stream: tokio::net::TcpStream,
    tm: u64,
    buf_size: usize
) -> Result<(), std::io::Error> {
    let mut target_stream = tokio::net::TcpStream::connect(SocketAddr::new(target, port)).await?;

    let (ch_snd, mut ch_rcv) = tokio::sync::mpsc::channel(1);
    let timeout_handler = async move {
        loop {
            match timeout(std::time::Duration::from_secs(tm), async {
                ch_rcv.recv().await
            })
            .await
            {
                Err(_) => break,
                Ok(None) => break,
                _ => continue,
            };
        }

        Err::<(), tokio::io::Error>(tokio::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Connection idle timeout",
        ))
    };

    let mut tcpwriter_target = crate::tcp::TcpBiGeneric {
        io: std::pin::Pin::new(&mut target_stream),
        signal: ch_snd.clone(),
    };

    let mut tcpwriter_server = crate::tcp::TcpBiGeneric {
        io: std::pin::Pin::new(&mut stream),
        signal: ch_snd,
    };

    tokio::try_join!(
        timeout_handler,
        tokio::io::copy_bidirectional_with_sizes(&mut tcpwriter_server, &mut tcpwriter_target, buf_size, buf_size)
    )?;
    Ok(())
}