extern crate bytes;
use mio::udp::UdpSocket;
use std::net::SocketAddr;
use request::base::*;
use server_mio::RequestCtx;
//
// Encapsulates the components of a dns request and response over Udp.
//
pub struct UdpRequest {
    upstream_socket: Option<UdpSocket>,
    client_addr: SocketAddr,
    inner: RequestBase,
}

impl Request<UdpRequest> for UdpRequest {
    fn new_with(client_addr: SocketAddr, request: RequestBase) -> UdpRequest {
        return UdpRequest {
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
        self.upstream_socket = UdpSocket::v4().ok();
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
            Some(ref sock) => {
                match sock.recv_from(&mut buf) {
                    Ok(Some((count, _))) => self.inner.on_receive(ctx, count, &buf),
                    Ok(None) => debug!("No data received on upstream_socket. {:?}", ctx.token),
                    Err(e) => self.inner.on_receive_err(ctx, e)
                }
            },
            None => {}
        }
    }

    fn forward(&mut self, ctx: &mut RequestCtx) {
        
        //TODO: error on fail to create upstream socket
        match self.upstream_socket {
            Some(ref sock) => {
                match sock.send_to(&mut self.inner.query_buf.as_slice(), &self.inner.params.upstream_addr) {
                      Ok(Some(count)) => self.inner.on_forward(ctx, count, sock),
                      Ok(None) => debug!("0 bytes sent. Staying in same state {:?}", ctx.token),
                      Err(e) => self.inner.on_forward_err(ctx, e)
                  }
            },
            None => {}
        }
    }
}

impl UdpRequest {

    pub fn send(&self, socket: &UdpSocket) {
        match self.inner.response_buf {
            Some(ref response) => {
                info!("{:?} bytes to send", response.len());
                match socket.send_to(&mut &response.as_slice(), &self.client_addr) {
                    Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
                    Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
                }
            }
            None => error!("Trying to send before a response has been buffered."),
        }
    }
}
