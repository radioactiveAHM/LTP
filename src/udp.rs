use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use tokio::{sync::Mutex, time::timeout};

type LiveConnections = Arc<Mutex<HashMap<SocketAddr, tokio::sync::mpsc::Sender<Vec<u8>>>>>;

pub async fn udp_listen_handler(
    target: IpAddr,
    timeout_dur: u64,
    inbound: SocketAddr,
    log_error: bool,
    buf_size: usize,
    udp_channel_buffer_size: usize,
) {
    let udp = Arc::new(tokio::net::UdpSocket::bind(inbound).await.unwrap());
    println!("UDP listening on {}", inbound);

    let live: LiveConnections = Arc::new(Mutex::new(HashMap::new()));
    // accept udp datagram
    let mut buff = vec![0; buf_size * 1024];
    loop {
        if let Ok((datagram_len, addr)) = udp.recv_from(&mut buff).await {
            if let Some(ch) = live.lock().await.get(&addr) {
                let _ = ch.send(buff[..datagram_len].to_owned()).await;
            } else {
                let (ch_snd, mut ch_rcv) = tokio::sync::mpsc::channel(udp_channel_buffer_size);
                let _ = ch_snd.send(buff[..datagram_len].to_owned()).await;
                live.lock().await.insert(addr, ch_snd);

                let udp = udp.clone();
                let live = live.clone();
                tokio::spawn(async move {
                    let target_udp = tokio::net::UdpSocket::bind(get_unspecified(&target))
                        .await
                        .unwrap();
                    target_udp
                        .connect(SocketAddr::new(target, inbound.port()))
                        .await
                        .unwrap();
                    let mut buff = vec![0; buf_size * 1024];
                    loop {
                        match timeout(std::time::Duration::from_secs(timeout_dur), async {
                            tokio::select! {
                                ch_inbound = channel_recv(&mut ch_rcv, &target_udp) => {
                                    ch_inbound
                                },

                                udp_inbound = target_udp.recv(&mut buff) => {
                                    match udp_inbound {
                                        Ok(dgram_len)=> udp.send_to(&buff[..dgram_len], addr).await,
                                        Err(e) => Err(e)
                                    }
                                }
                            }
                        })
                        .await
                        {
                            Err(_) => {
                                // Connection timeout
                                if log_error && let Ok(local_addr) = target_udp.local_addr() {
                                    println!("UDP {}: IdleTimeout", local_addr);
                                }
                                live.lock().await.remove(&addr);
                                break;
                            }
                            Ok(Err(e)) => {
                                // Connection error
                                if log_error && let Ok(local_addr) = target_udp.local_addr() {
                                    println!("UDP {}: {e}", local_addr);
                                }
                                live.lock().await.remove(&addr);
                                break;
                            }
                            _ => (),
                        };
                    }
                });
            }
        }
    }
}

fn get_unspecified(addr: &IpAddr) -> SocketAddr {
    if addr.is_ipv4() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
    } else {
        SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
    }
}

async fn channel_recv(
    r: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
    udp: &tokio::net::UdpSocket,
) -> tokio::io::Result<usize> {
    if let Some(dgram) = r.recv().await {
        Ok(udp.send(&dgram).await?)
    } else {
        Err(tokio::io::Error::other("Channel closed"))
    }
}
