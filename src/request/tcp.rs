
use mio::tcp::{TcpStream};
use std::net::SocketAddr;
use std::io::{Read, Write};
use request::base::*;
use server_mio::RequestCtx;

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    client_addr: SocketAddr,
    base: RequestBase,
}

impl Request<TcpRequest> for TcpRequest {
    fn new_with(client_addr: SocketAddr, request: RequestBase) -> TcpRequest {
        return TcpRequest {
            upstream_socket: None,
            client_addr: client_addr,
            base: request,
        };
    }

    fn get(&self) -> &RequestBase {
        &self.base
    }

    fn get_mut(&mut self) -> &mut RequestBase {
        &mut self.base
    }

    fn accept(&mut self, ctx: &mut RequestCtx) {
        let addr = self.base.params.upstream_addr;
        match TcpStream::connect(&addr) {
            Ok(sock) => {
                self.base.accept(ctx, &sock);
                self.upstream_socket = Some(sock);
            },
            Err(e) => self.base.error_with(format!("Failed to connect to {:?} {:?}", addr, e))
        }
    }

    fn receive(&mut self, ctx: &mut RequestCtx) {
        debug_assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];
        match self.upstream_socket {
            Some(ref mut sock) => {
                match sock.read(&mut buf) {
                    Ok(count) => self.base.on_receive(ctx, count, &buf),
                    Err(e) => self.base.on_receive_err(ctx, e)
                }
            }
            None => error!("tcp receive"),
        }
    }

    fn forward(&mut self, ctx: &mut RequestCtx) {
        debug_assert!(ctx.events.is_writable());
        match self.upstream_socket {
            Some(ref mut sock) => {
                // prefix with length
                let len = self.base.query_buf.len() as u8;
                debug!("{:?} bytes to send (+2b prefix)", len);
                Self::prefix_with_length(&mut self.base.query_buf);
                match sock.write_all(&mut self.base.query_buf.as_slice()) {
                    Ok(_) => self.base.on_forward(ctx, len as usize, sock),
                    Err(e) => self.base.on_forward_err(ctx, e)
                }
            },
            None => error!("tcp forward")
        }
    }
}

impl TcpRequest {

    fn prefix_with_length(buf: &mut Vec<u8>) {
        //TCP responses are prefixed with a 2-byte length
        let len = buf.len() as u8;
        buf.insert(0, len);
        buf.insert(0, 0);
    }

    pub fn send(&self, socket: &mut TcpStream) {
        match self.base.response_buf {
            Some(ref response) => {
                debug!("{:?} bytes to send", response.len());
                match socket.write(&mut &response.as_slice()) {
                    Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
                    Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
                }
            }
            None => error!("Trying to send before a response has been buffered."),
        }
    }
}
