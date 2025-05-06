#[derive(serde::Deserialize)]
pub struct Configuration {
    pub listen_ip: std::net::IpAddr,
    pub ports: Vec<u16>,
    pub udp_ports: Vec<u16>,
    pub target: std::net::IpAddr,
    pub udptimeout: u64,
    pub tcptimeout: u64,
    pub tcp_buffer_size: Option<usize>
}

pub fn load_config() -> Configuration {
    serde_json::from_slice(&std::fs::read("config.json").expect("Can not read config file"))
        .expect("Malformed config file")
}
