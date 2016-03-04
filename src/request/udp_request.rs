extern crate bytes;
use mio::EventSet;
use mio::udp::UdpSocket;
use std::net::SocketAddr;
use dns::dns_entities::DnsMessage;
use request::request_base::{RequestBase, RequestState};
use server_mio::RequestContext;
//
// Encapsulates the components of a dns request and response over Udp.
//
pub struct UdpRequest {
    upstream_socket: Option<UdpSocket>,
    pub client_addr: SocketAddr,
    pub inner: RequestBase,
}

impl UdpRequest {
    pub fn new(client_addr: SocketAddr, request: RequestBase) -> UdpRequest {
        return UdpRequest {
            upstream_socket: None,
            client_addr: client_addr,
            inner: request,
        };
    }

    fn accept(&mut self, ctx: &mut RequestContext) {
        debug_assert!(ctx.events.is_readable());
        self.inner.set_state(RequestState::Accepted);
        //todo: if need to forward...

        self.upstream_socket = UdpSocket::v4().ok();
        debug!("upstream created");
        match self.upstream_socket {
            Some(ref sock) => self.inner.register_upstream(ctx, EventSet::writable(), sock),
            None => error!("No upstream socket")
        }
    }

    fn forward(&mut self, ctx: &mut RequestContext) {
        debug!("Forwarding...");
        debug_assert!(ctx.events.is_writable());
        //TODO: error on fail to create upstream socket
        match self.upstream_socket {
            Some(ref sock) => {
                match sock.send_to(&mut self.inner.query_buf.as_slice(), &self.inner.params.upstream_addr) {
                      Ok(Some(_)) => {
                          self.inner.set_state(RequestState::Forwarded);
                          self.inner.register_upstream(ctx, EventSet::readable(), sock);
                          // TODO: No, don't just timeout forwarded requests, time out the whole request,
                          // be it cached, authorative or forwarded
                          self.inner.set_timeout(ctx);
                      }
                      Ok(None) => debug!("0 bytes sent. Staying in same state {:?}", ctx.token),
                      Err(e) => {
                          self.inner.error_with(format!("Failed to write to upstream_socket. {:?} {:?}",
                                                        e,
                                                        ctx.token))
                      }
                  }
            },
            None => {}
        }
    }

    fn receive(&mut self, ctx: &mut RequestContext) {
        assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];
        match self.upstream_socket {
            Some(ref sock) => {
                match sock.recv_from(&mut buf) {
                    Ok(Some((count, addr))) => {
                        debug!("Received {} bytes from {:?}", count, addr);
                        trace!("{:#?}", DnsMessage::parse(&buf));
                        self.inner.buffer_response(&buf, count);
                        self.inner.clear_timeout(ctx);
                        self.inner.set_state(RequestState::ResponseReceived);
                    }
                    Ok(None) => debug!("No data received on upstream_socket. {:?}", ctx.token),
                    Err(e) => {
                        self.inner.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e));
                        self.inner.clear_timeout(ctx);
                    }
                }
            },
            None => {}
        }
    }

    pub fn ready(&mut self, ctx: &mut RequestContext) {
        debug!("State {:?} {:?} {:?}",
               self.inner.state,
               ctx.token,
               ctx.events);
        // todo: authorative? cached? forward?
        match self.inner.state {
            RequestState::New => self.accept(ctx),
            RequestState::Accepted => self.forward(ctx),
            RequestState::Forwarded => self.receive(ctx),
            _ => debug!("Nothing to do for this state {:?}", self.inner.state),
        }
    }

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
