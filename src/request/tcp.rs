
use mio::tcp::{TcpStream};
use std::net::SocketAddr;
use std::io::{Read, Write};
use request::base::*;
use server_mio::RequestCtx;

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    client_addr: SocketAddr,
    inner: RequestBase,
}

impl Request<TcpRequest> for TcpRequest {
    fn new_with(client_addr: SocketAddr, request: RequestBase) -> TcpRequest {
        return TcpRequest {
            upstream_socket: None,
            client_addr: client_addr,
            inner: request,
        };
    }

    fn get(&self) -> &RequestBase {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut RequestBase {
        &mut self.inner
    }

    fn accept(&mut self, ctx: &mut RequestCtx) {
        self.upstream_socket = TcpStream::connect(&self.inner.params.upstream_addr).ok();
        //TODO: error on fail to create upstream socket
        debug!("upstream created");
        match self.upstream_socket {
            Some(ref sock) => self.inner.accept(ctx, sock),
            None => {}
        }
    }

    fn receive(&mut self, ctx: &mut RequestCtx) {
        assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];

        match self.upstream_socket {
            Some(ref mut socket) => {
                match socket.read(&mut buf) {
                    Ok(count) => self.inner.on_receive(ctx, count, &buf),
                    Err(e) => self.inner.on_receive_err(ctx, e)
                }
            }
            None => error!("tcp stream"),
        }
    }

    fn forward(&mut self, ctx: &mut RequestCtx) {
        debug!("Forwarding... {:?} bytes to forward", self.inner.query_buf.len());
        debug_assert!(ctx.events.is_writable());

        match self.upstream_socket {
            Some(ref mut sock) => {
                // prefix with length
                let len = self.inner.query_buf.len() as u8;
                self.inner.query_buf.insert(0, len);
                self.inner.query_buf.insert(0, 0);

                info!("{:?} bytes to send", self.inner.query_buf.len());
                match sock.write_all(&mut self.inner.query_buf.as_slice()) {
                      Ok(_) => self.inner.on_forward(ctx, len as usize, sock),
                      Err(e) => self.inner.on_forward_err(ctx, e)
                }
            },
            None => error!("tcp upstream socket")
        }
    }
}

impl TcpRequest {

    pub fn send(&self, socket: &mut TcpStream) {
        match self.inner.response_buf {
            Some(ref response) => {
                info!("{:?} bytes to send", response.len());

                match socket.write(&mut &response.as_slice()) {
                    Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
                    Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
                }
            }
            None => error!("Trying to send before a response has been buffered."),
        }
    }
}
