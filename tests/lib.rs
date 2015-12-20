extern crate koala_dns;
use koala_dns::server::*;
use std::net::SocketAddr;
use std::str::FromStr;


#[test]
#[ignore(message="will hang until stop implemented")]
fn start_server() {
    let server = Server {
        port: 12345,
        upstream_server: SocketAddr::from_str("8.8.8.8:53").unwrap(),
        timeout: 2000,
    };
    server.start();
}
