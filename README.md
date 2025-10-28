<div align="center">
  <img src="./icon/sample-x256.png" alt="Logo">

  <h3 align="center">Lazy Tcp Port-Forwarding</h3>
</div>

<hr>

<!-- ABOUT THE PROJECT -->
## Extreme Performance Async Multi-Threaded TCP and UDP Port Forwarding in Rust with Tokio Runtime

## Key Features

* Asynchronous and Multi-Threaded: The port forwarding engine is designed to handle multiple connections concurrently, making efficient use of system resources.
* TCP and UDP Support: Forward traffic for both TCP and UDP protocols seamlessly.
* Configurable: Easily customize port mappings, source and destination addresses, and other settings.
* Performance Optimized: Leverage Rust’s memory safety and Tokio’s async capabilities to achieve outstanding performance.

## Built With

* [![Rust][Rust]][Rust-url]
* [![Tokio][Tokio]][Tokio-url]

## Build

1. Clone the repository:
    1. `git clone https://github.com/radioactiveAHM/LTP.git`
    2. `cd LTP`
2. Build LTP:
    * `cargo build --release`
3. The compiled binary will be located at `/target/release/LTP`.

## Usage

1. Configure `config.json`: Edit the config.json file to set up any necessary configurations for LTP.
2. Run LTP.

## Config

```json
{
 "log_error": false,
 "listen_ip": "0.0.0.0",
 "tcp_proxy": ["1.1.1.1:443"],
 "udp_proxy": [],
 "udptimeout": 120,
 "tcptimeout": 300,
 "tcp_fill_buffer": false,
 "tcp_buffer_size": null,
 "udp_buffer_size": null,
 "udp_channel_buffer_size": 12
}
```

## License

Distributed under the Apache License Version 2.0. See `LICENSE` for more information.

<!-- MARKDOWN LINKS & IMAGES -->
[Rust]: https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white
[Rust-url]: https://www.rust-lang.org/
[Tokio]: https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=tokiodotrs&logoColor=white
[Tokio-url]: https://tokio.rs/
