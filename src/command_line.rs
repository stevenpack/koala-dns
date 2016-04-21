// to remove warnings when building both the bin and lib
#![allow(dead_code)]

use getopts::Options;
use std::env;
use std::process;
use std::net::SocketAddr;
use std::str::FromStr;


pub struct Config {
    pub port: u32,
    pub server: SocketAddr,
    pub timeout: u64,
    pub master_file: String
}

pub fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("p", "port", "Port to listen on", "53");
    opts.optopt("s", "server", "Upstream server", "8.8.8.8:53");
    opts.optopt("t",
                "timeout",
                "Upstream response timeout in milliseconds",
                "1000");
    opts.optopt("m", "master_file", "Path to the master file", "master.txt");
    opts.optflag("h", "help", "print this help menu");

    debug!("Parsing command line options");
    let matches = opts.parse(&args[1..])
                      .unwrap_or_else(|e| panic!("Failed to parse command line options {}", e));
    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }

    // Port
    debug!("Parsing port...");
    let mut port = "53".to_string();
    if matches.opt_present("p") {
        port = matches.opt_str("p").unwrap();
    }
    debug!("Port is {:?}", port);
    let port_num = port.parse::<u32>().unwrap_or_else(|e| panic!("port must be an integer. {}", e));

    // Upstream
    debug!("Parsing upstream server...");
    let server = "8.8.8.8:53";
    let upstream_server: SocketAddr;
    if matches.opt_present("s") {
        let s = matches.opt_str("s").unwrap();
        upstream_server = SocketAddr::from_str(s.as_str())
                              .unwrap_or_else(|e| panic!("Failed to parse upstream server. {}", e));
    } else {
        upstream_server = SocketAddr::from_str(server).unwrap();
    }

    // Timeout
    debug!("Parsing timeout...");
    let mut timeout = "1000".to_string();
    if matches.opt_present("t") {
        timeout = matches.opt_str("t").unwrap();
    }
    debug!("Timeout is {:?}", timeout);
    let timeout_num = timeout.parse::<u64>()
                             .unwrap_or_else(|e| panic!("timeout must be an integer. {}", e));

    //Master file
    debug!("Parsing master_file...");
    let mut master_file = String::from("master.txt");
    if matches.opt_present("m") {
        master_file = matches.opt_str("m").unwrap();
    }


    return Config {
        port: port_num,
        server: upstream_server,
        timeout: timeout_num,
        master_file: master_file
    };
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}
