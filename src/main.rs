extern crate getopts;
extern crate mio;

mod command_line;
mod server;

use server::Start;

fn main() {
    
    let config = command_line::parse_args();
    let server = server::Server {port: config.port};
    server.start();
}
