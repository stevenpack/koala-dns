extern crate mio;
use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::tcp::TcpStream;
use std::io::{Read, Write};
use mio::udp::UdpSocket;
use std::net::SocketAddr;

pub struct Socket {
    udp_socket: Option<UdpSocket>,
    tcp_socket: Option<TcpStream>,
}

impl Socket {
    pub fn new(udp_socket: Option<UdpSocket>, tcp_socket: Option<TcpStream>) -> Socket {
        return Socket {
            udp_socket: udp_socket,
            tcp_socket: tcp_socket,
        };
    }
}

pub trait SocketOps {
    fn evented(&self) -> &Evented;
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
    fn recv_from(&mut self, buf: &mut [u8]) -> Option<usize> {
        if self.udp_socket.is_some() {
            match self.udp_socket {
                Some(ref socket) => {
                    match socket.recv_from(buf) {
                        Ok(Some((cnt, addr))) => return Some(cnt),
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
