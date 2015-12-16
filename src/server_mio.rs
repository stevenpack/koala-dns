extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler, Timeout};
use mio::udp::UdpSocket;
use mio::util::Slab;
use std::net::SocketAddr;
use udp_request::{UdpRequest, RequestState};
use dns_entities::DnsMessage;

pub struct MioServer {
    udp_server: UdpSocket,
    upstream_server: SocketAddr,
    timeout: u64,
    requests: Slab<UdpRequest>,
    responses: Vec<UdpRequest>
}

const UDP_SERVER_TOKEN: Token = Token(1);

impl Handler for MioServer {
    type Timeout = Token;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet) {
        match token {
            UDP_SERVER_TOKEN => self.server_ready(event_loop, events),
            client_token => self.upstream_ready(event_loop, events, client_token)
        }
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, event_loop: &mut EventLoop<Self>, timeout: Self::Timeout) {
        info!("Got timeout: {:?}", timeout);
        self.remove_request(timeout);
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

    fn upstream_ready(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet, token: Token) {
        self.requests[token].socket_ready(event_loop, token, events);
        if self.requests[token].state == RequestState::ResponseReceived {
            self.queue_response(token);
            self.reregister_server(event_loop, EventSet::readable() | EventSet::writable());
        }
    }

    fn queue_response(&mut self, token: Token) {
        match self.remove_request(token) {
            Some(request) => {
                self.responses.push(request);
                debug!("Added {:?} to pending replies", token);
            },
            None => error!("Failed to remove request and queue response. {:?}", token)
        }
    }

    fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        debug!("Re-registered: {:?} with {:?}", UDP_SERVER_TOKEN, events);
        let _ = event_loop.reregister(&self.udp_server,
                                      UDP_SERVER_TOKEN,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
    }

    fn remove_request(&mut self, token: Token) -> Option<UdpRequest> {
        match self.requests.remove(token) {
            Some(request) => {
                debug!("Removed {:?} from pending requests.", token);
                return Some(request);
            },
            None => warn!("No request found {:?}", token)
        }
        return None;
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

    fn receive(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        //2.3.4 Size Limits from RFC1035
        let mut buf = vec![0;512];
        match socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {}", count, addr);
                //trace!("{:?}", buf);
                buf.truncate(count);
                return Some((addr, buf))
            },
            Ok(None) => { debug!("Server socket not ready to receive"); return None},
            Err(e) => { error!("Receive failed {:?}", e); return None},
        };
    }

    fn add_transaction(&mut self, addr: SocketAddr, bytes: &[u8]) -> Option<Token> {
        let upstream_server = self.upstream_server;
        let timeout_ms = self.timeout;
        //todo: lose the vecs
        let mut buf = Vec::<u8>::with_capacity(512);
        buf.push_all(bytes);
        match self.requests.insert(UdpRequest::new(addr, upstream_server, buf, timeout_ms)) {
            Ok(new_tok) => return Some(new_tok),
            Err(_) => { error!("Unable to start new transaction. Add to slab failed."); return None;}
        };
    }

   fn accept(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        let new_tok = self.receive(&self.udp_server)
                      .and_then(|(addr, buf)| self.add_transaction(addr, buf.as_slice()));

        if new_tok.is_some() {
            {
                //todo: improve
                debug!("{:#?}", DnsMessage::parse(&self.requests.get(new_tok.unwrap()).unwrap().query_buf));
            }
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

    pub fn start(address: SocketAddr, upstream_server: SocketAddr, timeout: u64) {
        let udp_server = MioServer::bind_udp(address);

        let mut event_loop = EventLoop::new().unwrap();
        let _ = event_loop.register(&udp_server,
                                    UDP_SERVER_TOKEN,
                                    EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot());

        let max_connections = u16::max_value() as usize;
        let mut mio_server = MioServer {
            udp_server: udp_server,
            upstream_server: upstream_server,
            timeout: timeout,
            requests: Slab::new_starting_at(Token(2), max_connections),
            responses: Vec::<UdpRequest>::new()
        };
        info!("Mio server running...");
        let _ = event_loop.run(&mut mio_server);
        drop(mio_server.udp_server);
    }
}


#[cfg(test)]
mod tests {
    //use super::*;
    #[test]
    #[ignore]
    fn it_works() {}
}
