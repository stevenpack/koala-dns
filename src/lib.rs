//to support both a lib and bin build
#![feature(test)]
extern crate getopts;
extern crate mio;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate test;

pub mod server;

mod dns;
mod command_line;
mod server_mio;
mod request;

mod buf;
