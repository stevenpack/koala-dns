extern crate mio;
use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::tcp::TcpStream;
use std::io::{Read, Write};
struct Socket;
trait SocketOps {
    // fn register(&self) -> &Evented;
    fn connect();
    fn accept();
    fn recv_from(&mut self, buf: &mut [u8]) -> usize;
    fn send_to(&self, buf: &[u8]) -> usize;
}

struct TcpSocketWrapper {
    socket: Option<TcpStream>,
}

impl TcpSocketWrapper {
    fn new() -> TcpSocketWrapper {
        TcpSocketWrapper { socket: None }
    }
}
impl SocketOps for TcpSocketWrapper {
    fn connect() {
        // socket.
    }
    fn accept() {}

    fn recv_from(&mut self, buf: &mut [u8]) -> usize {
        match self.socket {
            Some(ref mut sock) => {
                match sock.read(buf) {
                    Ok(bytes) => return bytes,
                    Err(err) => error!("{}", err),
                }
            }
            None => {}
        }
        0
    }

    fn send_to(&self, buf: &[u8]) -> usize {
        0
    }
}
