#![feature(convert)]
extern crate getopts;
extern crate mio;

mod command_line;
mod server_mio;

use server_mio::Start;

fn main() {

    let config = command_line::parse_args();
    let server = server_mio::Server { port: config.port };
    server.start();
}
