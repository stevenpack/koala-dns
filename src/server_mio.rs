extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt};
use mio::udp::UdpSocket;
use mio::util::Slab;
use std::net::SocketAddr;

pub struct MioServer {
    udp_server: UdpSocket,
    udp_transactions: Slab<UdpTransaction>,
    upstream_server: SocketAddr
}

impl MioServer {

    fn receive(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        let mut buf = Vec::with_capacity(4096);
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
        match self.udp_transactions.insert_with(|tok,| UdpTransaction::new(addr, tok, upstream_server, buf)) {
            Some(new_tok) => return Some(new_tok),            
            None => { error!("Unable to start new transaction. Add to slab failed."); return None;}
        };
    }

   fn accept_udp_connection(&mut self, event_loop: &mut EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        let new_tok = self.receive(&self.udp_server)
                      .and_then(|(addr, buf)| self.add_transaction(addr, buf));
        
        if new_tok.is_some() {
            self.udp_transactions[new_tok.unwrap()].socket_ready(event_loop, token, events, &self.udp_server);
        }
    }

    fn bind_udp(address: SocketAddr) -> mio::udp::UdpSocket {
        info!("Binding UDP to {:?}", address);
        let udp_socket = mio::udp::UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create udp socket {}", e));
        let _ = udp_socket.bind(&address).unwrap_or_else(|e| panic!("Failed to bind udp socket. Error was {}", e));
        return udp_socket;
    }

    pub fn start(address: SocketAddr, upstream_server: SocketAddr) {
        let udp_server = MioServer::bind_udp(address);

        let mut event_loop = mio::EventLoop::new().unwrap();
        let _ = event_loop.register_opt(&udp_server,
                                        UDP_SERVER_TOKEN,
                                        mio::EventSet::readable(),
                                        mio::PollOpt::edge() | mio::PollOpt::oneshot());

        let max_connections = u16::max_value() as usize;
        let mut mio_server = MioServer {
            udp_server: udp_server,
            udp_transactions: Slab::new_starting_at(mio::Token(2), max_connections),
            upstream_server: upstream_server
        };
        info!("Mio server running...");
        let _ = event_loop.run(&mut mio_server);
        drop(mio_server.udp_server);
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
enum State {
    AcceptClient,
    ForwardRequest,
    ReceiveResponse,
    AnswerClient,
    Close,
}
//
// Encapsulates the components of a Resolution request and response over Udp.
//
#[derive(Debug)]
struct UdpTransaction {
    state: State,
    client_addr: SocketAddr,
    upstream_token: Token,
    upstream_socket: UdpSocket,
    upstream_addr: SocketAddr,
    query_buf: Vec<u8>,
    response_buf: Vec<u8>,
}

impl UdpTransaction {
    fn new(client_addr: SocketAddr, upstream_token: Token, upstream_addr: SocketAddr, query_buf: Vec<u8>) -> UdpTransaction {
        debug!("New UDP transaction: {:?}", upstream_token);
        return UdpTransaction {
            state: State::AcceptClient,
            client_addr: client_addr,
            upstream_token: upstream_token,            
            upstream_socket: UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create UDP socket {:?}", e)),
            upstream_addr: upstream_addr,
            query_buf: query_buf,
            response_buf: Vec::<u8>::new(),
        };
    }

    fn set_state(&mut self, state: State) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    #[allow(unused_variables)]
    fn accept_client(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
        assert!(events.is_readable());

        //todo: carllerche
        //Re-register server socket to accept new connections while this
        //this transaction is processed
        self.reregister_server(event_loop, udp_server, UDP_SERVER_TOKEN);

        //Register upstream socket to write
        let _ = event_loop.register_opt(&self.upstream_socket,
                                    self.upstream_token,
                                    EventSet::writable(),
                                    PollOpt::edge() | PollOpt::oneshot());

        self.set_state(State::ForwardRequest);
    }

    #[allow(unused_variables)]
    fn forward_request(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
         assert!(events.is_writable());
        //todo: upstream
        // todo: what to pass the query buf around as?
        match self.upstream_socket.send_to(&mut self::bytes::SliceBuf::wrap(self.query_buf.as_slice()), &self.upstream_addr) {
            Ok(Some(_)) => {
                //todo log bytes
                //todo: timeout
                self.set_state(State::ReceiveResponse);
                let _ = event_loop.register_opt(&self.upstream_socket,
                                                token,
                                                EventSet::readable(),
                                                PollOpt::edge() | PollOpt::oneshot());
            }
            Ok(None) => debug!("Failed to send. Expect writable event to fire again still in the same state. {:?}", token),
            Err(e) => error!("Failed to write to upstream_socket. {:?}. {:?}", token, e),
        }            
    }

    fn receive_response(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
        assert!(events.is_readable());
        //todo: higher perf buffer?
        let mut buf = Vec::<u8>::new();
        match self.upstream_socket.recv_from(&mut buf) {
            Ok(Some(addr)) => {
                debug!("Received {} bytes from {:?}", buf.len(), addr);
                trace!("{:?}", buf);
                self.response_buf = buf;
                self.set_state(State::AnswerClient);
                // register the server socket to write.
                // todo: does this stop us reading...
                // carlleche? should we stay readable to accept new connetions?
                let _ = event_loop.register_opt(udp_server,
                                                token,
                                                EventSet::writable(),
                                                PollOpt::edge() | PollOpt::oneshot());
            }
            Ok(None) => debug!("No data received on upstream_socket. {:?}", token),
            Err(e) => println!("Receive failed on {:?}. {:?}", token, e),
        }
    }

    fn answer_client(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
        assert!(events.is_writable());
        match udp_server.send_to(&mut self::bytes::SliceBuf::wrap(&self.response_buf), &self.client_addr) {
            Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, token),
            Err(e) => error!("Failed to send. {:?} Error was {:?}", token, e),
        }
        debug!("UdpTransaction complete. {:?}", token);
        self.set_state(State::Close);
        // register the server socket to read
        // TODO: were we not accepting clients this whole time?
        self.reregister_server(event_loop, udp_server, UDP_SERVER_TOKEN);
    }

    fn socket_ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {

        debug!("Socket Event. State: {:?} {:?} EventSet: {:?}", self.state, token, events);
        match self.state {
            State::AcceptClient => self.accept_client(event_loop, token, events, udp_server),
            State::ForwardRequest => self.forward_request(event_loop, token, events, udp_server),
            State::ReceiveResponse => self.receive_response(event_loop, token, events, udp_server),
            State::AnswerClient => self.answer_client(event_loop, token, events, udp_server),
            State::Close => error!("Should not be firing socket events when closed. {:?}", token)            
        }
    }

    fn reregister_server(&self,
                         event_loop: &mut EventLoop<MioServer>,
                         evented: &mio::Evented,
                         token: Token) {
        debug!("Re-registered: {:?}", token);
        let _ = event_loop.reregister(evented,
                                      token,
                                      EventSet::readable(),
                                      PollOpt::edge() | PollOpt::oneshot());
    }
}

const UDP_SERVER_TOKEN: mio::Token = mio::Token(1);

impl mio::Handler for MioServer {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        match token {
            UDP_SERVER_TOKEN => self.accept_udp_connection(event_loop, token, events),
            client_token => {
                // Handling events on the upstream socket, or a write on the server socket
                self.udp_transactions[client_token].socket_ready(event_loop, client_token, events, &self.udp_server);
                if self.udp_transactions[client_token].state == State::Close {
                    //todo: doing this causes a panic after the handler?
                    //let _ = event_loop.deregister(&self.udp_transactions[token].upstream_socket);
                    self.udp_transactions.remove(client_token);
                    debug!("Removed {:?}", client_token);
                }
            }
        }
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
