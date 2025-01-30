mod config;
mod udp;

use tokio::net::TcpListener;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

#[tokio::main]
async fn main() {
    let conf = config::load_config();
    let ports: Vec<u16> = conf.ports;

    let ip = {
        if conf.localhost {
            (
                Ipv4Addr::new(127, 0, 0, 1),
                Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            )
        } else {
            (
                Ipv4Addr::new(0, 0, 0, 0),
                Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            )
        }
    };

    let mut tasks = Vec::new();
    // Generate tasks for tcp
    for port in ports {
        // IPv4
        tasks.push(tokio::spawn(async move {
            listen_handler(
                port,
                conf.target,
                SocketAddr::V4(SocketAddrV4::new(ip.0, port)),
            )
            .await;
        }));
        // IPv6
        if conf.ipv6 {
            tasks.push(tokio::spawn(async move {
                listen_handler(
                    port,
                    conf.target,
                    SocketAddr::V6(SocketAddrV6::new(ip.1, port, 0, 0)),
                )
                .await;
            }));
        }
    }

    // Generate tasks for udp
    let ports: Vec<u16> = conf.udp_ports;
    for port in ports {
        // IPv4
        tasks.push(tokio::spawn(async move {
            udp::udp_listen_handler(
                port,
                conf.target,
                conf.udptimeout,
                SocketAddr::V4(SocketAddrV4::new(ip.0, port)),
            )
            .await;
        }));
        // IPv6
        if conf.ipv6 {
            tasks.push(tokio::spawn(async move {
                udp::udp_listen_handler(
                    port,
                    conf.target,
                    conf.udptimeout,
                    SocketAddr::V6(SocketAddrV6::new(ip.1, port, 0, 0)),
                )
                .await;
            }));
        }
    }

    for atask in tasks {
        if atask.await.is_err() {
            println!("Task Error");
        }
    }
}

async fn listen_handler(port: u16, target: IpAddr, inbound: SocketAddr) {
    let tcp = TcpListener::bind(inbound).await.unwrap();

    // accept streams
    loop {
        if let Ok(stream) = tcp.accept().await {
            tokio::spawn(async move {
                if let Err(e) = stream_handler(port, target, stream.0).await {
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
) -> Result<(), std::io::Error> {
    let (mut server_r, mut server_w) = stream.split();

    // connect to target
    let mut target_stream = tokio::net::TcpStream::connect(SocketAddr::new(target, port)).await?;
    let (mut target_r, mut target_w) = target_stream.split();

    tokio::try_join!(
        tokio::io::copy(&mut target_r, &mut server_w),
        tokio::io::copy(&mut server_r, &mut target_w),
    )?;
    Ok(())
}