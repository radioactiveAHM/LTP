use std::{net::SocketAddr, sync::Arc};

use tokio::{sync::{mpsc::channel, Mutex}, time::timeout};


pub async fn udp_listen_handler(port: u16, target: String, timeout_dur:u64, inbound: SocketAddr) {
    println!("listening on {}", port);
    // panic if can't listen on the IP+Port
    // udp inbound
    let udp = Arc::new(tokio::net::UdpSocket::bind(inbound).await.unwrap());

    // list of live connections
    let live: Arc<Mutex<Vec<(tokio::sync::mpsc::Sender<Vec<u8>>, SocketAddr)>>> = Arc::new(Mutex::new(Vec::with_capacity(5)));
    // accept udp datagram
    loop {
        let mut buff = [0;16384];
        if let Ok((datagram_len, addr)) = udp.recv_from(&mut buff).await {
            // check if addr is in live list and get it.
            let ch = live.lock().await.iter().find_map(|conn|{
                if conn.1==addr{
                    Some(conn.0.clone())
                }else {
                    None
                }
            });

            // if addr is in live
            if ch.is_some(){
                ch.unwrap().send(buff[..datagram_len].to_vec()).await.unwrap_or(());
            }else {                
                // if addr is not in live
                let (ch_snd, mut ch_rcv) = channel(1);
                live.lock().await.push((ch_snd.clone(), addr.clone()));
                
                let target = target.clone();
                let addr = addr.clone();
                let udp = udp.clone();
                let live = live.clone();
                let inbound_ip_version = inbound.is_ipv4();
                tokio::spawn(async move {
                    let target_socket_addr_v = {
                        if inbound_ip_version {
                            "0.0.0.0:0"
                        } else {
                            "[::]:0"
                        }
                    };
                    let target_udp = tokio::net::UdpSocket::bind(target_socket_addr_v).await.unwrap();
                    target_udp.connect(format!("{target}:{port}")).await.unwrap();
                    loop {
                        let mut buff = [0;16384];
                        let tm = timeout(std::time::Duration::from_secs(timeout_dur), async {
                            return tokio::select! {
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
                            };
                        }).await;

                        if tm.is_err() {
                            // Connection timeout
                            let mut live_lock = live.lock().await;
                            if let Some(index) = live_lock.iter().position(|conn| conn.1==addr){
                                live_lock.swap_remove(index);
                            }
                            break;
                        }else if tm.unwrap().is_err() {
                            // Connection closed
                            let mut live_lock = live.lock().await;
                            if let Some(index) = live_lock.iter().position(|conn| conn.1==addr){
                                live_lock.swap_remove(index);
                            }
                            break;
                        }
                    }
                });
    
                ch_snd.send(buff[..datagram_len].to_vec()).await.unwrap_or(());
            }
        }
    }
}