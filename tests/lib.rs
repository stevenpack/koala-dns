extern crate koala_dns;
use koala_dns::server::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;
use std::process::Command;

#[test]
// #[ignore(message="will hang until stop implemented")]
fn round_trip() {
    let mut server = Server {
        port: 12345,
        upstream_server: SocketAddr::from_str("8.8.8.8:53").unwrap(),
        timeout: 2000,
        sender: None,
    };
    let run_handle = server.begin_start();
    thread::spawn(|| run_handle.join());
    let sleep_ms = 2000;
    println!("Sleeping {:?}...", sleep_ms);
    thread::sleep_ms(sleep_ms);

    let output = Command::new("dig")
                     .arg("yahoo.com")
                     .arg("@127.0.0.1")
                     .arg("-p")
                     .arg("12345")
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    // println!("process exited with: {}", output);
    let output_str = String::from_utf8(output.stdout).unwrap();

    println!("{:?}", output_str);

    assert!(output_str.contains(";; ANSWER SECTION:"));
    assert!(output_str.contains("yahoo.com."));

    server.stop();
}
