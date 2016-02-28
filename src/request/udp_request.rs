extern crate bytes;
use mio::{Token, EventSet, Timeout, Handler, PollOpt};
use mio::udp::UdpSocket;
use std::net::SocketAddr;
use dns::dns_entities::DnsMessage;
use dns::dns_entities::DnsHeader;
use request::request_base::{RequestBase, RequestState};
use server_mio::RequestContext;
//
// Encapsulates the components of a dns request and response over Udp.
//
// #[derive(Debug)]
pub struct UdpRequest {
    upstream_socket: UdpSocket,
    client_addr: SocketAddr,
    inner: RequestBase,
}


impl UdpRequest {
    pub fn new(client_addr: SocketAddr, request: RequestBase) -> Option<UdpRequest> {
        // debug!("New UDP transaction: {:?}", upstream_token);

        match UdpSocket::v4() {
            Ok(sock) => {
                return Some(UdpRequest {
                    upstream_socket: sock,
                    client_addr: client_addr,
                    inner: request,
                });
            }
            Err(e) => {
                error!("Failed to create UDP socket. {}", e);
                None
            }
        }
    }

    fn set_state(&mut self, state: RequestState) {
        debug!("{:?} -> {:?}", self.inner.state, state);
        self.inner.state = state;
    }

    fn set_timeout_handle(&mut self, timeout: Timeout) {
        self.inner.timeout_handle = Some(timeout);
    }

    fn register_upstream(&mut self, ctx: &mut RequestContext, events: EventSet) {

        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        ctx.event_loop
           .register(&self.upstream_socket, ctx.token, events, poll_opt)
           .unwrap_or_else(|e| {
               self.error_with(format!("Failed to register upstream socket. {}", e))
           });
    }

    pub fn on_timeout(&mut self, token: Token) {
        self.error_with(format!("{:?} timed out", token));
    }

    fn set_timeout(&mut self, ctx: &mut RequestContext) {
        match ctx.event_loop.timeout_ms(ctx.token, self.inner.params.timeout) {
            Ok(t) => self.set_timeout_handle(t),
            Err(e) => error!("Failed to schedule timeout for {:?}. {:?}", ctx.token, e),
        }
    }

    pub fn clear_timeout(&mut self, ctx: &mut RequestContext) {
        match self.inner.timeout_handle {
            Some(handle) => {
                if ctx.event_loop.clear_timeout(handle) {
                    debug!("Timeout cleared for {:?}", ctx.token);
                } else {
                    warn!("Could not clear timeout for {:?}", ctx.token);
                }
            }
            None => warn!("Timeout handle not present"),
        }

    }

    fn accept(&mut self, ctx: &mut RequestContext) {
        debug_assert!(ctx.events.is_readable());
        self.set_state(RequestState::Accepted);
        self.register_upstream(ctx, EventSet::writable());
    }

    fn forward(&mut self, ctx: &mut RequestContext) {
        debug_assert!(ctx.events.is_writable());
        match self.upstream_socket
                  .send_to(&mut self.inner.query_buf.as_slice(),
                           &self.inner.params.upstream_addr) {
            Ok(Some(_)) => {
                self.set_state(RequestState::Forwarded);
                self.register_upstream(ctx, EventSet::readable());
                // TODO: No, don't just timeout forwarded requests, time out the whole request,
                // be it cached, authorative or forwarded
                self.set_timeout(ctx);
            }
            Ok(None) => debug!("0 bytes sent. Staying in same state {:?}", ctx.token),
            Err(e) => {
                self.error_with(format!("Failed to write to upstream_socket. {:?} {:?}",
                                        e,
                                        ctx.token))
            }
        }
    }

    fn receive(&mut self, ctx: &mut RequestContext) {
        assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];
        match self.upstream_socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {:?}", count, addr);
                trace!("{:#?}", DnsMessage::parse(&buf));
                self.buffer_response(&buf, count);
                self.clear_timeout(ctx);
                self.set_state(RequestState::ResponseReceived);
            }
            Ok(None) => debug!("No data received on upstream_socket. {:?}", ctx.token),
            Err(e) => {
                self.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e));
                self.clear_timeout(ctx);
            }
        }
    }

    fn buffer_response(&mut self, buf: &[u8], count: usize) {
        let mut response = Vec::with_capacity(count);
        response.extend_from_slice(&buf);
        response.truncate(count);
        self.inner.response_buf = Some(response);
    }

    pub fn ready(&mut self, ctx: &mut RequestContext) {
        debug!("State {:?} {:?} {:?}",
               self.inner.state,
               ctx.token,
               ctx.events);
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

    pub fn error_with(&mut self, err_msg: String) {
        self.set_state(RequestState::Error);
        info!("{}", err_msg);
        let req = DnsMessage::parse(&self.inner.query_buf);
        let reply = DnsHeader::new_error(req.header, 2);
        let vec = reply.to_bytes();
        self.inner.response_buf = Some(vec);
    }

    pub fn has_reply(&self) -> bool {
        return self.inner.response_buf.is_some();
    }
}
