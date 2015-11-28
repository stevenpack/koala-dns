use getopts::Options;
use std::env;
use std::process;
use std::net::SocketAddr;
use std::str::FromStr;

pub struct Config {
    pub port: i32,
    pub server: SocketAddr
}

pub fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("p", "port", "Port to listen on", "53");
    opts.optopt("s", "server", "Upstream server", "8.8.8.8:53");
    opts.optflag("h", "help", "print this help menu");

    debug!("Parsing command line options");
    let matches = opts.parse(&args[1..]).unwrap_or_else(|e| panic!("Failed to parse command line options {}", e));
    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }

    //Port
    debug!("Parsing port...");
    let mut port = "53".to_string();
    if matches.opt_present("p") {
        port = matches.opt_str("p").unwrap();
    }
    debug!("Port is {:?}", port);
    let port_num = port.parse::<i32>().unwrap_or_else(|e| panic!("port must be an integer. {}", e));

    //Upstream
    debug!("Parsing upstream server...");
    let server = "8.8.8.8:53";
    let upstream_server: SocketAddr;
    if matches.opt_present("s") {
        let s = matches.opt_str("s").unwrap();
        upstream_server = SocketAddr::from_str(s.as_str()).unwrap_or_else(|e| panic!("Failed to parse upstream server. {}", e));
    } else {
        upstream_server = SocketAddr::from_str(server).unwrap();
    }

    return Config 
    {
        port: port_num,
        server: upstream_server 
    };
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}
