extern crate bytes;
use mio::{Token, EventSet, Timeout, EventLoop, Handler, PollOpt};
use mio::udp::UdpSocket;
use std::net::SocketAddr;
use dns::dns_entities::DnsMessage;
use dns::dns_entities::DnsHeader;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived,
    Error,
}

pub struct RequestBase {
    state: RequestState,
    query_buf: Vec<u8>,
    response_buf: Option<Vec<u8>>,
    timeout_handle: Option<Timeout>,

    // Separate? This is only required for upstream
    params: RequestParams,
}

pub struct RequestParams {
    pub timeout: u64,
    pub upstream_addr: SocketAddr,
}

impl RequestBase {
    pub fn new(query_buf: Vec<u8>, params: RequestParams) -> RequestBase {
        return RequestBase {
            state: RequestState::New,
            query_buf: query_buf,
            response_buf: None,
            timeout_handle: None,
            params: params,
        };
    }
}
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

    fn register_upstream<T>(&mut self,
                            event_loop: &mut EventLoop<T>,
                            events: EventSet,
                            token: Token)
        where T: Handler
    {
        event_loop.register(&self.upstream_socket,
                            token,
                            events,
                            PollOpt::edge() | PollOpt::oneshot())
                  .unwrap_or_else(|e| error!("Failed to register upstream socket. {}", e));
        // todo fail the reqest
    }

    pub fn on_timeout(&mut self, token: Token) {
        self.error_with(format!("{:?} timed out", token));
    }

    fn set_timeout<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token)
        where T: Handler<Timeout = Token>
    {
        match event_loop.timeout_ms(token, self.inner.params.timeout) {
            Ok(t) => self.set_timeout_handle(t),
            Err(e) => error!("Failed to schedule timeout for {:?}. {:?}", token, e),
        }
    }

    pub fn clear_timeout<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token)
        where T: Handler<Timeout = Token>
    {
        match self.inner.timeout_handle {
            Some(handle) => {
                if event_loop.clear_timeout(handle) {
                    debug!("Timeout cleared for {:?}", token);
                } else {
                    warn!("Could not clear timeout for {:?}", token);
                }
            }
            None => warn!("Timeout handle not present"),
        }

    }

    fn accept<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token, events: EventSet)
        where T: Handler<Timeout = Token>
    {
        debug_assert!(events.is_readable());
        self.set_state(RequestState::Accepted);
        self.register_upstream(event_loop, EventSet::writable(), token);
    }

    fn forward<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token, events: EventSet)
        where T: Handler<Timeout = Token>
    {
        debug_assert!(events.is_writable());
        match self.upstream_socket
                  .send_to(&mut self.inner.query_buf.as_slice(),
                           &self.inner.params.upstream_addr) {
            Ok(Some(_)) => {
                self.set_state(RequestState::Forwarded);
                self.register_upstream(event_loop, EventSet::readable(), token);
                // TODO: No, don't just timeout forwarded requests, time out the whole request,
                // be it cached, authorative or forwarded
                self.set_timeout(event_loop, token);
            }
            Ok(None) => debug!("0 bytes sent. Staying in same state {:?}", token),
            Err(e) => {
                self.error_with(format!("Failed to write to upstream_socket. {:?} {:?}", e, token))
            }
        }
    }

    fn receive<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token, events: EventSet)
        where T: Handler<Timeout = Token>
    {
        assert!(events.is_readable());
        let mut buf = [0; 4096];
        match self.upstream_socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {:?}", count, addr);
                trace!("{:#?}", DnsMessage::parse(&buf));
                self.buffer_response(&buf, count);
                self.clear_timeout(event_loop, token);
                self.set_state(RequestState::ResponseReceived);
            }
            Ok(None) => debug!("No data received on upstream_socket. {:?}", token),
            Err(e) => {
                self.error_with(format!("Receive failed on {:?}. {:?}", token, e));
                self.clear_timeout(event_loop, token);
            }
        }
    }

    fn buffer_response(&mut self, buf: &[u8], count: usize) {
        let mut response = Vec::with_capacity(count);
        response.extend_from_slice(&buf);
        response.truncate(count);
        self.inner.response_buf = Some(response);
    }

    pub fn ready<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token, events: EventSet)
        where T: Handler<Timeout = Token>
    {
        debug!("State {:?} {:?} {:?}", self.inner.state, token, events);
        match self.inner.state {
            RequestState::New => self.accept(event_loop, token, events),
            RequestState::Accepted => self.forward(event_loop, token, events),
            RequestState::Forwarded => self.receive(event_loop, token, events),
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
