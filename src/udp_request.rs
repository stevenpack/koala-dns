extern crate bytes;
use mio::{Token, EventSet, Timeout, EventLoop, Handler, PollOpt};
use mio::udp::UdpSocket;
use std::net::SocketAddr;
use dns_entities::DnsMessage;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived,
}

//
// Encapsulates the components of a Resolution request and response over Udp.
//
// #[derive(Debug)]
pub struct UdpRequest {
    pub state: RequestState,
    upstream_socket: UdpSocket,
    timeout_ms: u64,
    timeout_handle: Option<Timeout>,
    client_addr: SocketAddr,
    upstream_addr: SocketAddr,
    pub query_buf: Vec<u8>,
    response_buf: Vec<u8>,
}


impl UdpRequest {
    pub fn new(client_addr: SocketAddr,
               upstream_addr: SocketAddr,
               query_buf: Vec<u8>,
               timeout: u64)
               -> UdpRequest {
        // debug!("New UDP transaction: {:?}", upstream_token);
        return UdpRequest {
            state: RequestState::New,
            client_addr: client_addr,
            // upstream_token: upstream_token,
            // todo: handle this by Option<> and error! the request but do not panic, or accepting in ctor
            upstream_socket: UdpSocket::v4()
                                 .unwrap_or_else(|e| panic!("Failed to create UDP socket {:?}", e)),
            upstream_addr: upstream_addr,
            query_buf: query_buf,
            response_buf: Vec::<u8>::with_capacity(4096),
            timeout_ms: timeout,
            timeout_handle: None,
        };
    }

    fn set_state(&mut self, state: RequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    fn set_timeout_handle(&mut self, timeout: Timeout) {
        self.timeout_handle = Some(timeout);
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

    fn set_timeout<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token)
        where T: Handler<Timeout = Token>
    {
        match event_loop.timeout_ms(token, self.timeout_ms) {
            Ok(t) => self.set_timeout_handle(t),
            Err(e) => error!("Failed to schedule timeout for {:?}. {:?}", token, e),
        }
    }

    fn clear_timeout<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token)
        where T: Handler<Timeout = Token>
    {
        if event_loop.clear_timeout(self.timeout_handle.unwrap()) {
            debug!("Timeout cleared for {:?}", token);
        } else {
            debug!("Could not clear timeout for {:?}", token);
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
        match self.upstream_socket.send_to(&mut self.query_buf.as_slice(), &self.upstream_addr) {
            Ok(Some(_)) => {
                self.set_state(RequestState::Forwarded);
            }
            Ok(None) => {
                debug!("Failed to send. Expect writable event to fire again still in the same \
                        state. {:?}",
                       token)
            }
            Err(e) => error!("Failed to write to upstream_socket. {:?}. {:?}", token, e),
        }
        self.register_upstream(event_loop, EventSet::readable(), token);
        self.set_timeout(event_loop, token);
    }

    fn receive<T>(&mut self, event_loop: &mut EventLoop<T>, token: Token, events: EventSet)
        where T: Handler<Timeout = Token>
    {
        assert!(events.is_readable());
        // todo: higher perf buffer?
        let mut buf: [u8; 512] = [0; 512];
        match self.upstream_socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {:?}", buf.len(), addr);
                // todo: lose the vecs
                self.response_buf.push_all(&buf);
                self.response_buf.truncate(count);
                // self.response_buf = buf;
                self.set_state(RequestState::ResponseReceived);
                self.clear_timeout(event_loop, token);
                debug!("{:#?}", DnsMessage::parse(&buf));
            }
            Ok(None) => debug!("No data received on upstream_socket. {:?}", token),
            Err(e) => println!("Receive failed on {:?}. {:?}", token, e),
        }
    }
    pub fn socket_ready<T>(&mut self,
                           event_loop: &mut EventLoop<T>,
                           token: Token,
                           events: EventSet)
        where T: Handler<Timeout = Token>
    {
        debug!("Socket Event. State: {:?} {:?} EventSet: {:?}",
               self.state,
               token,
               events);
        match self.state {
            RequestState::New => self.accept(event_loop, token, events),
            RequestState::Accepted => self.forward(event_loop, token, events),
            RequestState::Forwarded => self.receive(event_loop, token, events),
            RequestState::ResponseReceived => {
                error!("Unexpected socket event for {:?}. State {:?}",
                       token,
                       self.state)
            }
        }
    }

    pub fn send(&self, socket: &UdpSocket) {
        match socket.send_to(&mut &self.response_buf.as_slice(), &self.client_addr) {
            Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
            Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
        }
    }
}
