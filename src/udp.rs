use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use tokio::{
    sync::{mpsc::channel, Mutex},
    time::timeout,
};

type LiveConnections = Arc<Mutex<Vec<(tokio::sync::mpsc::Sender<Vec<u8>>, SocketAddr)>>>;

pub async fn udp_listen_handler(target: IpAddr, timeout_dur: u64, inbound: SocketAddr) {
    println!("UDP listening on {}", inbound);

    // panic if can't listen on the IP+Port
    // udp inbound
    let udp = Arc::new(tokio::net::UdpSocket::bind(inbound).await.unwrap());

    // list of live connections
    let live: LiveConnections = Arc::new(Mutex::new(Vec::with_capacity(5)));
    // accept udp datagram
    let mut buff = [0; 8196];
    loop {
        if let Ok((datagram_len, addr)) = udp.recv_from(&mut buff).await {
            // check if addr is in live list and get it.
            let ch = live.lock().await.iter().find_map(|conn| {
                if conn.1 == addr {
                    Some(conn.0.clone())
                } else {
                    None
                }
            });

            // if addr is in live
            if ch.is_some() {
                let _ = ch.unwrap().send(buff[..datagram_len].to_owned()).await;
            } else {
                // if addr is not in live
                let (ch_snd, mut ch_rcv) = channel(1);
                live.lock().await.push((ch_snd.clone(), addr));

                let udp = udp.clone();
                let live = live.clone();
                tokio::spawn(async move {
                    let target_udp = tokio::net::UdpSocket::bind(SocketAddr::new(inbound.ip(), 0))
                        .await
                        .unwrap();
                    target_udp
                        .connect(SocketAddr::new(target, inbound.port()))
                        .await
                        .unwrap();
                    let mut buff = [0; 8196];
                    loop {
                        let tm = timeout(std::time::Duration::from_secs(timeout_dur), async {
                            tokio::select! {
                                ch_inbound = ch_rcv.recv() => {
                                    if let Some(dgram)=ch_inbound{
                                        target_udp.send(&dgram).await
                                    }else {
                                        Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted))
                                    }
                                },

                                udp_inbound = target_udp.recv(&mut buff) => {
                                    if let Ok(dgram_len)=udp_inbound{
                                        udp.send_to(&buff[..dgram_len], addr).await
                                    }else {
                                        Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted))
                                    }
                                }
                            }
                        }).await;

                        if tm.is_err() {
                            // Connection timeout
                            let mut live_lock = live.lock().await;
                            if let Some(index) = live_lock.iter().position(|conn| conn.1 == addr) {
                                live_lock.swap_remove(index);
                            }
                            break;
                        } else if tm.unwrap().is_err() {
                            // Connection closed
                            let mut live_lock = live.lock().await;
                            if let Some(index) = live_lock.iter().position(|conn| conn.1 == addr) {
                                live_lock.swap_remove(index);
                            }
                            break;
                        }
                    }
                });

                let _ = ch_snd.send(buff[..datagram_len].to_owned()).await;
            }
        }
    }
}
