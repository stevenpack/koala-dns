#![feature(convert)]
extern crate getopts;
extern crate mio;
#[macro_use] 
extern crate log;
extern crate env_logger;

mod command_line;
mod server_mio;

use server_mio::Start;

fn main() {
	env_logger::init().unwrap_or_else(|err| println!("Failed to initialize logger. {:?}", err));

    let config = command_line::parse_args();
    let server = server_mio::Server { port: config.port };
    server.start();
}
