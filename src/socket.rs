extern crate mio;
use mio::Evented;
use mio::tcp::TcpStream;
use std::io::{Read, Write};
use mio::udp::UdpSocket;
use std::net::SocketAddr;

pub struct Socket {
    udp_socket: Option<UdpSocket>,
    tcp_socket: Option<TcpStream>,
}

//
// Common interface to recv/send to a socket, be it UDP or TCP.
//
impl Socket {
    pub fn new() -> Socket {
        return Socket {
            udp_socket: None,
            tcp_socket: None,
        };
    }
}

pub trait SocketOps {
    fn evented(&self) -> &Evented;
    fn connect(&mut self, addr: SocketAddr);
    fn is_connected(&self) -> bool;
    fn recv_from(&mut self, buf: &mut [u8]) -> Option<usize>;
    fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> Option<usize>;
}

impl SocketOps for Socket {
    fn evented(&self) -> &Evented {
        if self.udp_socket.is_some() {
            return self.udp_socket.as_ref().unwrap();
        }
        return self.tcp_socket.as_ref().unwrap();
    }
    fn connect(&mut self, addr: SocketAddr) {
        debug!("Connecting to {:?}", addr);
        // TODO: this will be None... ServerToken gets passed into Request
        if self.udp_socket.is_some() {
            self.udp_socket = UdpSocket::v4().ok();
            return;
        }
        if self.tcp_socket.is_some() {
            self.tcp_socket = TcpStream::connect(&addr).ok();
        }
    }
    fn is_connected(&self) -> bool {
        return self.udp_socket.is_some() || self.tcp_socket.is_some();
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> Option<usize> {
        if self.udp_socket.is_some() {
            match self.udp_socket {
                Some(ref socket) => {
                    match socket.recv_from(buf) {
                        Ok(Some((cnt, _))) => return Some(cnt),
                        Ok(None) => return None,
                        Err(err) => {
                            error!("Failed to receive on UDP. {}", err);
                            return None;
                        }
                    }
                }
                None => {}
            }
        }
        if self.tcp_socket.is_some() {
            match self.tcp_socket {
                Some(ref mut socket) => {
                    match socket.read(buf) {
                        Ok(cnt) => return Some(cnt),
                        Err(err) => {
                            error!("Failed to receive on TCP. {}", err);
                            return None;
                        }
                    }
                }
                None => error!("tcp stream"),
            }
        }
        error!("No UDP or TCP socket to recv");
        None
    }

    fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> Option<usize> {
        if self.udp_socket.is_some() {
            match self.udp_socket {
                Some(ref socket) => {
                    match socket.send_to(buf, &addr) {
                        Ok(Some(cnt)) => return Some(cnt),
                        Ok(None) => {
                            error!("udp_socket send returned None");
                            return None;
                        }
                        Err(err) => {
                            error!("udp_socket send failed {}", err);
                            return None;
                        }
                    }
                }
                None => {}
            }
        }
        if self.tcp_socket.is_some() {
            match self.tcp_socket {
                Some(ref mut socket) => {
                    match socket.write(buf) {
                        Ok(cnt) => return Some(cnt),
                        Err(err) => {
                            error!("tcp_socket send failed {}", err);
                            return None;
                        }
                    }
                }
                None => {}
            }
        }
        error!("No UDP or TCP socket to send");
        None
    }
}
