#![feature(test)]
#![feature(associated_consts)]
extern crate getopts;
extern crate mio;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate test;
extern crate time;
//extern crate koala_dns;

mod command_line;
mod server;
mod server_mio;
mod dns;
mod request;
mod buf;
mod servers;
mod cache;
use server::ServerOps;

fn main() {
	env_logger::init().unwrap_or_else(|err| println!("Failed to initialize logger. {:?}", err));

    let config = command_line::parse_args();
    let mut server = server::Server::new(config.port, config.server, config.timeout);
    server.start();
}
