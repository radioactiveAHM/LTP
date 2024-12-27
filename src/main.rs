mod config;
mod udp;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    select,
    time::timeout,
};

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

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
                conf.timeout,
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
                    conf.timeout,
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
                conf.timeout,
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
                    conf.timeout,
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

async fn listen_handler(port: u16, target: SocketAddr, timeout_dur: u64, inbound: SocketAddr) {
    let tcp = TcpListener::bind(inbound).await.unwrap();

    // accept streams
    loop {
        if let Ok(stream) = tcp.accept().await {
            tokio::spawn(async move {
                if let Err(e) = stream_handler(port, target, timeout_dur, stream.0).await {
                    println!("{e}");
                }
            });
        }
    }
}

async fn stream_handler(
    port: u16,
    target: SocketAddr,
    timeout_dur: u64,
    mut stream: tokio::net::TcpStream,
) -> Result<(), std::io::Error> {
    let (mut server_r, mut server_w) = stream.split();

    // connect to target
    let mut target_stream = tokio::net::TcpStream::connect(format!("{target}:{port}")).await?;
    let (mut target_r, mut target_w) = target_stream.split();

    let mut target_stat = 0u8;
    let mut server_stat = 0u8;
    let mut buff1 = [0; 8196];
    let mut buff2 = [0; 8196];
    loop {
        if target_stat == 5 || server_stat == 5 {
            // target or server closed conn
            break;
        }
        let to = timeout(std::time::Duration::from_secs(timeout_dur), async {
            let stat = select! {
                buff_len = server_r.read(&mut buff1) => {
                    if let Ok(size) = buff_len {
                        target_stat = 0;
                        (Stats::Server,target_w.write(&buff1[..size]).await)
                    } else {
                        (Stats::Server,Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted)))
                    }
                }
                buff_len = target_r.read(&mut buff2) => {
                    if let Ok(size) = buff_len {
                        server_stat = 0;
                        (Stats::Target,server_w.write(&buff2[..size]).await)
                    }else {
                        (Stats::Target,Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted)))
                    }
                }
            };
            match stat {
                (Stats::Target, Ok(size)) => {
                    if size == 0 {
                        target_stat += 1;
                    }
                }
                (Stats::Server, Ok(size)) => {
                    if size == 0 {
                        server_stat += 1;
                    }
                }
                (Stats::Target, Err(e)) => {
                    println!("{}", e);
                    target_stat = 5;
                }
                (Stats::Server, Err(e)) => {
                    println!("{}", e);
                    server_stat = 5;
                }
            }
        })
        .await;

        if to.is_err() {
            break;
        }
    }
    Ok(())
}

enum Stats {
    Server,
    Target,
}
