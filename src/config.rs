#[derive(serde::Deserialize)]
pub struct Configuration {
    pub log_error: bool,
    pub listen_ip: std::net::IpAddr,
    pub tcp_proxy: Vec<std::net::SocketAddr>,
    pub udp_proxy: Vec<std::net::SocketAddr>,
    pub tcptimeout: u64,
    pub udptimeout: u64,
    pub tcp_buffer_size: Option<usize>,
    pub udp_buffer_size: Option<usize>,
    pub udp_channel_buffer_size: usize,
}

pub fn load_config() -> Configuration {
    serde_json::from_slice(&std::fs::read("config.json").expect("Can not read config file"))
        .expect("Malformed config file")
}
