#![feature(test)]
#![feature(associated_consts)]
#![feature(plugin)]
// #![plugin(clippy)]
#[allow(unknown_lints)] //until can build clippy with nightly on travis
extern crate getopts;
extern crate mio;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate test;
extern crate time;

mod command_line;
mod server;
mod server_mio;
mod dns;
mod request;
mod buf;
mod servers;
mod cache;

use server::ServerOps;
use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;

fn main() {
	let format = |record: &LogRecord| {
        format!("{} {:5} - {}", time::now().asctime(), record.level(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap_or_else(|err| println!("Failed to initialize logger. {:?}", err));
	//env_logger::init().unwrap_or_else(|err| println!("Failed to initialize logger. {:?}", err));

    let config = command_line::parse_args();
    let mut server = server::Server::new(config.port, config.server, config.timeout, config.master_file);
    server.start();
}
