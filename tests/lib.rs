extern crate koala_dns;

use koala_dns::server::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;
use std::process::Command;

const GOOGLE_DNS: &'static str = "8.8.8.8:53";
const FAKE_DNS: &'static str = "8.8.8.8:9999";

#[test]
fn round_trip() {
    let mut server = build_with(12345, String::from_str(GOOGLE_DNS).unwrap_or_else(|e| panic!("failed to resolve address {:?}", e)), 2000);

    let output_str = start(&mut server);
    println!("{:?}", output_str);

    assert!(output_str.contains(";; ANSWER SECTION:"));
    assert!(output_str.contains("yahoo.com."));

    server.stop();
}

#[test]
fn timeout() {
    println!("Starting...");
    let mut server = build_with(12346, String::from_str(FAKE_DNS).unwrap_or_else(|e| panic!("failed to resolve address {:?}", e)), 200);
    println!("Post build with...");
    let output_str = start(&mut server);
    println!("output string {:?}", output_str);

    assert!(output_str.contains("status: SERVFAIL"));

    server.stop();
}

fn start(server: &mut Server) -> String {
    println!("Starting server...");
    let run_handle = server.begin_start();
    println!("begin_start returned. spawning thread");
    thread::spawn(|| run_handle.join());
    // let sleep_ms = 2000;
    // println!("Sleeping {:?}...", sleep_ms);
    // thread::sleep_ms(sleep_ms);
    println!("Creating and executing dig... with {}", server.port);
    let output = Command::new("dig")
                     .arg("yahoo.com")
                     .arg("@127.0.0.1")
                     .arg("-p")
                     .arg(format!("{}", server.port))
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    // println!("process exited with: {}", output);
    let output_str = String::from_utf8(output.stdout).unwrap_or_else(|e| panic!("utf8 parse failed {:?}", e));
    return output_str;
}

fn build_with(port: u32, server: String, timeout_ms: u64) -> Server {
    let server = Server::new(port,
                             SocketAddr::from_str(server.as_str()).unwrap_or_else(|e| panic!("Couldn't start server {:?}", e)),
                             timeout_ms);
    return server;
}
