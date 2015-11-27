use getopts::Options;
use std::env;
use std::process;

pub struct Config {
    pub port: i32,
}

pub fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("p", "port", "Port to listen on", "53");
    opts.optflag("h", "help", "print this help menu");

    println!("Parsing command ine options");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(e.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        process::exit(0);
    }

    println!("Parsing port...");

    let mut port = "53".to_string();
    if matches.opt_present("p") {
        port = matches.opt_str("p").unwrap();
    }

    println!("Port is {:?}", port);

    let p: i32 = match port.parse::<i32>() {
        Ok(x) => x,
        Err(e) => panic!("port must be an integer. {}", e),
    };

    return Config { port: p };
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}
