#![feature(convert)]
extern crate koala_dns;
use koala_dns::server::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;
use std::process::Command;

const GOOGLE_DNS: &'static str = "8.8.8.8:53";
const FAKE_DNS: &'static str = "8.8.8.8:9999";

#[test]
#[ignore]
// #[ignore(message="will hang until stop implemented")]
fn round_trip() {
    let mut server = build_with(String::from_str(GOOGLE_DNS).unwrap(), 2000);

    let output_str = start(&mut server);
    println!("{:?}", output_str);

    assert!(output_str.contains(";; ANSWER SECTION:"));
    assert!(output_str.contains("yahoo.com."));

    server.stop();
}

#[test]
fn timeout() {
    let mut server = build_with(String::from_str(FAKE_DNS).unwrap(), 200);

    let output_str = start(&mut server);
    println!("{:?}", output_str);

    assert!(output_str.contains(";; connection timed out"));

    server.stop();
}

fn start(server: &mut Server) -> String {
    let run_handle = server.begin_start();
    thread::spawn(|| run_handle.join());
    // let sleep_ms = 2000;
    // println!("Sleeping {:?}...", sleep_ms);
    // thread::sleep_ms(sleep_ms);

    let output = Command::new("dig")
                     .arg("yahoo.com")
                     .arg("@127.0.0.1")
                     .arg("-p")
                     .arg(format!("{}", server.port))
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    // println!("process exited with: {}", output);
    let output_str = String::from_utf8(output.stdout).unwrap();
    return output_str;
}

fn build_with(server: String, timeout_ms: u64) -> Server {
    let mut server = Server::new(12345,
                                 SocketAddr::from_str(server.as_str()).unwrap(),
                                 timeout_ms);
    return server;
}
