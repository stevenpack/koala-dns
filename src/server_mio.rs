extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::udp::UdpSocket;
use mio::util::Slab;
use std::net::SocketAddr;

pub struct MioServer {
    udp_server: UdpSocket,
    upstream_server: SocketAddr,
    requests: Slab<UdpRequest>,
    responses: Vec<UdpRequest>
}

#[derive(Debug)]
#[derive(PartialEq)]
enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived
}

//
// Encapsulates the components of a Resolution request and response over Udp.
//
#[derive(Debug)]
struct UdpRequest {
    state: RequestState,
    client_addr: SocketAddr,
    upstream_token: Token,
    upstream_socket: UdpSocket,
    upstream_addr: SocketAddr,
    query_buf: Vec<u8>,
    response_buf: Vec<u8>
}

impl UdpRequest {
    fn new(client_addr: SocketAddr, upstream_token: Token, upstream_addr: SocketAddr, query_buf: Vec<u8>) -> UdpRequest {
        debug!("New UDP transaction: {:?}", upstream_token);
        return UdpRequest {
            state: RequestState::New,
            client_addr: client_addr,
            upstream_token: upstream_token,
            upstream_socket: UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create UDP socket {:?}", e)),
            upstream_addr: upstream_addr,
            query_buf: query_buf,
            response_buf: Vec::<u8>::new()
        };
    }

    fn set_state(&mut self, state: RequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet) {
        debug!("Socket Event. State: {:?} {:?} EventSet: {:?}", self.state, token, events);
        match self.state {
            RequestState::New => {
                assert!(events.is_readable());
                //Register upstream socket to write
                let _ = event_loop.register_opt(&self.upstream_socket, self.upstream_token, EventSet::writable(), PollOpt::edge() | PollOpt::oneshot());
                self.set_state(RequestState::Accepted);
            },
            RequestState::Accepted => {
                assert!(events.is_writable());
                match self.upstream_socket.send_to(&mut self::bytes::SliceBuf::wrap(self.query_buf.as_slice()), &self.upstream_addr) {
                    Ok(Some(_)) => {
                        //todo: timeout
                        self.set_state(RequestState::Forwarded);
                        let _ = event_loop.register_opt(&self.upstream_socket, token, EventSet::readable(), PollOpt::edge() | PollOpt::oneshot());
                    }
                    Ok(None) => debug!("Failed to send. Expect writable event to fire again still in the same state. {:?}", token),
                    Err(e) => error!("Failed to write to upstream_socket. {:?}. {:?}", token, e),
                }
            },
            RequestState::Forwarded => {
                assert!(events.is_readable());
                //todo: higher perf buffer?
                let mut buf = Vec::<u8>::new();
                match self.upstream_socket.recv_from(&mut buf) {
                    Ok(Some(addr)) => {
                        debug!("Received {} bytes from {:?}", buf.len(), addr);
                        trace!("{:?}", buf);
                        self.response_buf = buf;
                        self.set_state(RequestState::ResponseReceived);
                    }
                    Ok(None) => debug!("No data received on upstream_socket. {:?}", token),
                    Err(e) => println!("Receive failed on {:?}. {:?}", token, e),
                }
            },
            _ => error!("Unexpected socket event for {:?}", token)
        }
    }

    fn send(&self, socket: &UdpSocket) {
        match socket.send_to(&mut self::bytes::SliceBuf::wrap(&self.response_buf.as_slice()), &self.client_addr) {
            Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
            Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
        }
    }
}

const UDP_SERVER_TOKEN: Token = Token(1);

impl Handler for MioServer {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet) {
        match token {
            UDP_SERVER_TOKEN => self.server_ready(event_loop, events),
            client_token => self.upstream_ready(event_loop, events, client_token)
        }
    }
}

impl MioServer {

    fn server_ready(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        if events.is_readable() {
            self.accept(event_loop, events);
        }
        if events.is_writable() {
            self.send_reply();
        }
        //We are always listening for new requests. The server socket will be regregistered
        //as writable if there are responses to write
        self.reregister_server(event_loop, EventSet::readable());
    }

    fn upstream_ready(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet, client_token: Token) {
        self.requests[client_token].ready(event_loop, client_token, events);
        if self.requests[client_token].state == RequestState::ResponseReceived {
            match self.requests.remove(client_token) {
                Some(request) => {
                    debug!("Removed {:?} from pending requests.", client_token);
                    self.responses.push(request);
                    debug!("Added {:?} to pending replies", client_token);
                },
                None => warn!("No request found {:?}", client_token)
            }
            self.reregister_server(event_loop, EventSet::readable() | EventSet::writable());
        }
    }

    fn send_reply(&mut self) {
        debug!("There are {} responses to send", self.responses.len());
        match self.responses.pop() {
            Some(reply) => {
                reply.send(&self.udp_server);
            },
            None => { warn!("Nothing to send.") }
        }
    }

    fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        debug!("Re-registered: {:?} with {:?}", UDP_SERVER_TOKEN, events);
        let _ = event_loop.reregister(&self.udp_server,
                                      UDP_SERVER_TOKEN,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
    }

    fn receive(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        let mut buf = Vec::with_capacity(1024);
        match socket.recv_from(&mut buf) {
            Ok(Some(addr)) => {
                debug!("Received {} bytes from {}", buf.len(), addr);
                return Some((addr, buf))
            },
            Ok(None) => { debug!("Server socket not ready to receive"); return None},
            Err(e) => { error!("Receive failed {:?}", e); return None},
        };
    }

    fn add_transaction(&mut self, addr: SocketAddr, buf: Vec<u8>) -> Option<Token> {
        let upstream_server = self.upstream_server;
        match self.requests.insert_with(|tok| UdpRequest::new(addr, tok, upstream_server, buf)) {
            Some(new_tok) => return Some(new_tok),
            None => { error!("Unable to start new transaction. Add to slab failed."); return None;}
        };
    }

   fn accept(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        let new_tok = self.receive(&self.udp_server)
                      .and_then(|(addr, buf)| self.add_transaction(addr, buf));

        if new_tok.is_some() {
            debug!("There are {:?} in-flight requests", self.requests.count());
            self.ready(event_loop, new_tok.unwrap(), events);
        } else {
            error!("Failed to add request. New Token was None");
        }
    }

    fn bind_udp(address: SocketAddr) -> UdpSocket {
        info!("Binding UDP to {:?}", address);
        let udp_socket = UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create udp socket {}", e));
        let _ = udp_socket.bind(&address).unwrap_or_else(|e| panic!("Failed to bind udp socket. Error was {}", e));
        return udp_socket;
    }

    pub fn start(address: SocketAddr, upstream_server: SocketAddr) {
        let udp_server = MioServer::bind_udp(address);

        let mut event_loop = EventLoop::new().unwrap();
        let _ = event_loop.register_opt(&udp_server,
                                        UDP_SERVER_TOKEN,
                                        EventSet::readable(),
                                        PollOpt::edge() | PollOpt::oneshot());

        let max_connections = u16::max_value() as usize;
        let mut mio_server = MioServer {
            udp_server: udp_server,
            requests: Slab::new_starting_at(Token(2), max_connections),
            upstream_server: upstream_server,
            responses: Vec::<UdpRequest>::new()
        };
        info!("Mio server running...");
        let _ = event_loop.run(&mut mio_server);
        drop(mio_server.udp_server);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use mio::{Evented, Token, EventLoop};
    use mio::udp::UdpSocket;
    use mio::util::Slab;
    use std::net::SocketAddr;
    use std::thread;
    use std::time::Duration;
    #[test]
    fn it_works() {}
}
