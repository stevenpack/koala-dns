//to support both a lib and bin build
#![feature(test)]
#![feature(associated_consts)]#![feature(plugin)]
#![plugin(clippy)]
#[allow(unknown_lints)] //until can build clippy with nightly on travis
extern crate getopts;
extern crate mio;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate test;
extern crate time;
//extern crate koala_dns;

mod command_line;
pub mod server;
mod server_mio;
mod dns;
mod request;
mod buf;
pub mod servers;
mod cache;
