#[derive(serde::Deserialize)]
pub struct Configuration {
    pub ports: Vec<u16>,
    pub udp_ports: Vec<u16>,
    pub target: String,
    pub timeout: u64,
    pub ipv6: bool,
    pub localhost: bool
}

pub fn load_config() -> Configuration {
    serde_json::from_slice(&std::fs::read("config.json").expect("Can not read config file"))
        .expect("Malformed config file")
}
