extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt};
use mio::udp::UdpSocket;
use mio::util::Slab;
use std::net::SocketAddr;
//use std::str::FromStr;
//use std::thread;
//use std::time::Duration;
//use std::io::{Read,Write};
use mio::buf::{SliceBuf};
/*
Public shown to main
*/            //todo: get guidance from carllerche on when you need to reregister
pub struct Server {
    pub port: i32,
}

pub trait Start {
    fn start(&self);
}

impl Start for Server {
    fn start(&self) {
        println!("Starting server on port {}", self.port);        
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();        
        MioServer::start(address);
    }    
}

/*
Internal
*/
struct MioServer {
    udp_server: UdpSocket,
    udp_transactions: Slab<UdpTransaction>,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum State {
    AcceptClient,
    ForwardRequest,
    ReceiveResponse,
    AnswerClient,
    Close
}
/*
Encapsulates the components of a Resolution request and response over Udp.
*/
#[derive(Debug)]
struct UdpTransaction {
    state: State,
    client_addr: SocketAddr,
    upstream_token: Token,
    upstream_socket: UdpSocket,    
    query_buf: Vec<u8>,
    response_buf: Vec<u8>
}

impl UdpTransaction {

    fn new(client_addr: SocketAddr, upstream_token: Token, query_buf: Vec<u8>) -> UdpTransaction {
        println!("New UDP transaction");
        return UdpTransaction {
            state: State::AcceptClient,
            client_addr: client_addr,
            upstream_token: upstream_token,
            //not ideal, might fail
            upstream_socket: UdpSocket::v4().unwrap(),    
            query_buf: query_buf,
            response_buf: Vec::<u8>::new()
        };
    }

    // fn get_state(&self) -> &State {
    //     return &self.state;
    // }

    fn change_state(&mut self, state: State) {
        self.state = state;
    }

    fn socket_ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
        
        println!("Socket Event. State: {:?} Token: {:?} EventSet: {:?}", self.state, token, events);
        //todo: extract to metods

        match self.state {
            State::AcceptClient => {
                assert!(events.is_readable());
                self.change_state(State::ForwardRequest);
                let _ = event_loop.register_opt(&self.upstream_socket, self.upstream_token, EventSet::writable(), PollOpt::edge() | PollOpt::oneshot());
            },
            State::ForwardRequest => {
                assert!(events.is_writable());
                let upstream_addr = format!("8.8.8.8:{:?}", 53).parse().unwrap();
                //todo: what to pass the query buf around as?
                match self.upstream_socket.send_to(&mut bytes::SliceBuf::wrap(self.query_buf.as_slice()), &upstream_addr) {
                    Ok(Some(_)) => {
                        //todo log bytes
                        self.change_state(State::ReceiveResponse);
                        let _ = event_loop.register_opt(&self.upstream_socket, token, EventSet::readable(), PollOpt::edge() | PollOpt::oneshot());
                    },
                    Ok(None) => {println!("Failed to send. What now? Event fires again..?")}
                    Err(e) => println!("Failed to write {:?}", e)
                    //todo: free resources
                }
            }
            State::ReceiveResponse => {
                assert!(events.is_readable());
                let mut buf = Vec::<u8>::new();
                match self.upstream_socket.recv_from(&mut buf) {
                    Ok(Some(addr)) => {            
                        println!("received data from {:?}. Maybe even a DNS reply?", addr);                        
                        println!("Looks like this: {:?}", buf);
                        self.response_buf = buf;
                        self.change_state(State::AnswerClient);
                        //register the server socket to write.
                        //todo: does this stop us reading... should we stay readable to accept new connetions?
                        let _ = event_loop.register_opt(udp_server, token, EventSet::writable(), PollOpt::edge() | PollOpt::oneshot());
                    },
                    Ok(None) => println!("Got no data"),
                    Err(e) => println!("Receive failed {:?}", e)
                }
            },
            State::AnswerClient => {
                assert!(events.is_writable());
                println!("READY TO WRITE TO THE CLIENT");
                match udp_server.send_to(&mut SliceBuf::wrap(&self.response_buf), &self.client_addr) {

                    Ok(Some(n)) => println!("{} bytes sent", n),
                    Ok(None) => println!("No bytes sent"),
                    Err(e) => println!("Failed to send. Error was {:?}", e)
                }        
                //todo: log bytes sent and failure
                println!("done");
                self.change_state(State::Close);
                //register the server socket to read
                self.reregister_server(event_loop, udp_server, UDP_SERVER_TOKEN);
            },
            State::Close => {
                panic!("Should not be firing socket events when closed. Token: {:?}", token);
            }
        }
    }


    fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, evented: &mio::Evented, token: Token) {
        println!("re-registered: {:?}", token);
        let _ = event_loop.reregister(evented, token, EventSet::readable(), PollOpt::edge() | PollOpt::oneshot());
    }
}



const UDP_SERVER_TOKEN: mio::Token = mio::Token(1);

impl mio::Handler for MioServer {
    type Timeout = ();
    type Message = ();

    #[allow(unused_variables)]
    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        match token {           
            UDP_SERVER_TOKEN => self.accept_udp_connection(event_loop, token, events),
            _ => {
                //Handling events on the upstream socket, or a write on the server socket
                self.udp_transactions[token].socket_ready(event_loop, token, events, &self.udp_server);
                if self.udp_transactions[token].state == State::Close {
                    self.udp_transactions.remove(token);
                    println!("Removed {:?}", token);
                }
           }
        }
    }
}

impl MioServer {
    fn accept_udp_connection(&mut self, event_loop: &mut EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        let mut buf = Vec::<u8>::new();
        match self.udp_server.recv_from(&mut buf) {
            Ok(Some(addr)) => {            
               match self.udp_transactions.insert_with(|tok| UdpTransaction::new(addr, tok, buf)) {
                    Some(new_tok) => {
                        println!("Token for this transaction is {:?}", new_tok);
                        self.udp_transactions[new_tok].socket_ready(event_loop, token, events, &self.udp_server);
                    },
                    None => {
                        println!("Unable to start new transaction. Add to slab failed.");
                    }
               }
            },
            Ok(None) => println!("Socket not ready to receive. Need to re-register?"),
            Err(e) => println!("Receive failed {:?}", e)
        };
    }

    fn bind_udp(address: SocketAddr) -> mio::udp::UdpSocket {
        println!("Binding UDP to {:?}", address);
        let udp_socket = mio::udp::UdpSocket::v4().unwrap();
        let _ = match udp_socket.bind(&address) {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to bind UDP. Error was {}", e);
            }
        };
        return udp_socket;
    }

    fn start(address: SocketAddr) {
        let udp_server = MioServer::bind_udp(address);

        let mut event_loop = mio::EventLoop::new().unwrap();
        let _ = event_loop.register_opt(&udp_server, UDP_SERVER_TOKEN, mio::EventSet::readable(), mio::PollOpt::edge() | mio::PollOpt::oneshot());

        println!("running mio server");
        //todo: strategy for number of connections.
        let mut mio_server = MioServer {
            udp_server: udp_server,
            udp_transactions: Slab::new_starting_at(mio::Token(2), 1024),
        };
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
    fn it_works() {
       
    }
}
    
   
