mod config;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    select,
    time::timeout,
};

#[tokio::main]
async fn main() {
    let conf = config::load_config();
    let ports: Vec<u16> = conf.ports;

    let mut tasks = Vec::new();
    // Generate tasks
    for port in ports {
        let target = conf.target.clone();
        let timeout_dur = conf.timeout.clone();
        tasks.push(tokio::spawn(async move {
            listen_handler(port, target, timeout_dur).await;
        }));
    }

    for atask in tasks {
        if atask.await.is_err() {
            println!("Task Error");
        }
    }
}

async fn listen_handler(port: u16, target: String, timeout_dur:u64) {
    // panic if can't listen on the IP+Port
    let tcp = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    // accept streams
    loop {
        let target = target.to_string();
        if let Ok(stream) = tcp.accept().await {
            tokio::spawn(async move {
                stream_handler(port, target, timeout_dur, stream.0).await;
            });
        }
    }
}

async fn stream_handler(port: u16, target: String, timeout_dur:u64, mut stream: tokio::net::TcpStream) {
    let (mut server_r, mut server_w) = stream.split();

    // connect to target
    // panic if failed
    let mut target_stream = tokio::net::TcpStream::connect(format!("{target}:{port}"))
        .await
        .unwrap();
    let (mut target_r, mut target_w) = target_stream.split();

    let mut target_stat = 0u8;
    let mut server_stat = 0u8;
    loop {
        if target_stat == 5 || server_stat == 5 {
            // target or server closed conn
            break;
        }
        let to = timeout(std::time::Duration::from_secs(timeout_dur), async {
            let mut buff1 = [0; 8196];
            let mut buff2 = [0; 8196];
            let stat = select! {
                buff_len = server_r.read(&mut buff1) => {
                    if let Ok(size) = buff_len {
                        target_stat = 0;
                        (Stats::Server,target_w.write(&buff1[..size]).await)
                    } else {
                        (Stats::Server,Err(std::io::Error::from(std::io::ErrorKind::InvalidData)))
                    }
                }
                buff_len = target_r.read(&mut buff2) => {
                    if let Ok(size) = buff_len {
                        server_stat = 0;
                        (Stats::Target,server_w.write(&buff2[..size]).await)
                    }else {
                        (Stats::Target,Err(std::io::Error::from(std::io::ErrorKind::InvalidData)))
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
}

enum Stats {
    Server,
    Target,
}