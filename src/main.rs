#![feature(convert)]
#![feature(vec_push_all)]
extern crate getopts;
extern crate mio;
#[macro_use]
extern crate log;
extern crate env_logger;

mod command_line;
mod server;
mod server_mio;
mod udp_request;
mod dns_parser;
mod byte_utils;

use server::Start;

fn main() {
	env_logger::init().unwrap_or_else(|err| println!("Failed to initialize logger. {:?}", err));

    let config = command_line::parse_args();
    let server = server::Server {
		 port: config.port,
		 upstream_server: config.server,
		 timeout: config.timeout
	  };
    server.start();
}
