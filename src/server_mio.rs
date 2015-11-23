extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt};
use mio::udp::UdpSocket;
use mio::util::Slab;
use std::net::SocketAddr;
//use std::thread;
//use std::time::Duration;
//use std::io::{Read,Write};
//use mio::buf::{Buf,ByteBuf};
/*
Public shown to main
*/            //todo: get guidance from carllerche on when you need to reregister
pub struct Server {
    pub port: i32,
}

pub trait Start {
    fn start(&self);
}

/*
Internal
*/
struct MioServer {
    udp_server: UdpSocket,
    rx_connections: Slab<InboundConnection>,
    tx_connections: Slab<OutboundConnection>
}

#[derive(Debug)]
struct InboundConnection {
    rx_token: Token,
    rx_addr: SocketAddr,
    query: Option<u32> //type: tbd
}

#[derive(Debug)]
enum OutboundState {
    UpstreamWrite,
    UpstreamRead,
    ClientWrite
}

#[derive(Debug)]
struct OutboundConnection {
    state: OutboundState,
    tx_token: Token,
    tx_socket: UdpSocket,
    query_buf: Vec<u8>,
    rx_token: Token,
    rx_addr: SocketAddr,
    response: Option<u32> //type:tbd

}

impl InboundConnection {
    fn new(rx_token: Token, rx_addr: SocketAddr) -> InboundConnection {
        let conn = InboundConnection {
            rx_token: rx_token,
            rx_addr: rx_addr,
            query: None
        };
        return conn;
    }

    fn socket_ready(&self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet) {
        println!("I'm ready to write the client some data!!!");
    }
}

impl OutboundConnection {

    fn new(tx_token: Token, tx_socket: UdpSocket, query_buf: Vec<u8>, rx_token: Token, rx_addr: SocketAddr) -> OutboundConnection {
        return OutboundConnection {
            state: OutboundState::UpstreamWrite,
            tx_token: tx_token,
            tx_socket: tx_socket,
            query_buf: query_buf,
            rx_token: rx_token,
            rx_addr: rx_addr,
            response: None
        }
    }

    fn change_state(&mut self, state: OutboundState) {
        self.state = state;
    }

    fn socket_ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: Token, events: EventSet, udp_server: &UdpSocket) {
        println!("I'm ready to write or read some data from upstream!");

        match self.state {
            OutboundState::UpstreamWrite => {
                assert!(events.is_writable());
                println!("And the winner is... WRITING. Read token is: {:?}. Write token is: {:?}", self.rx_token, self.tx_token);            
                let tx_addr = format!("8.8.8.8:{:?}", 53).parse().unwrap();
                //todo: what to pass the query buf around as?
                match self.tx_socket.send_to(&mut bytes::SliceBuf::wrap(self.query_buf.as_slice()), &tx_addr) {
                    Ok(Some(_)) => {
                        self.change_state(OutboundState::UpstreamRead);
                        println!("Transitioned to read");
                        let _ = event_loop.register_opt(&self.tx_socket, token, EventSet::readable(), PollOpt::edge() | PollOpt::oneshot());
                    },
                    Ok(None) => {println!("Failed to send. What now? Event fires again..?")}
                    Err(e) => println!("Failed to write {:?}", e)
                    //todo: free resources
                }
            }
            OutboundState::UpstreamRead => {
                assert!(events.is_readable());
                println!("And the winner is... READING");
                let mut buf = Vec::<u8>::new();
                match self.tx_socket.recv_from(&mut buf) {
                    Ok(Some(addr)) => {            
                        println!("received data from {:?}. Maybe even a DNS reply?", addr);                        
                        println!("Looks like this: {:?}", buf);
                        self.change_state(OutboundState::ClientWrite);
                        //todo: register event loop 
                        udp_server.send_to(&mut bytes::SliceBuf::wrap(buf.as_slice()), &self.rx_addr);        
                        println!("Wrote the reply?!?!?!");
                    },
                    Ok(None) => println!("Got no data"),
                    Err(e) => println!("Receive failed {:?}", e)
                }
            },
            OutboundState::ClientWrite => {
                //assert!();
            }
        }
    }
}

impl Start for Server {
    fn start(&self) {
        println!("Starting server on port {}", self.port);        
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();        
        let udp_server = bind_udp(address);
        start(udp_server);
    }    
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

const UDP_SERVER_TOKEN: mio::Token = mio::Token(1);

impl mio::Handler for MioServer {
    type Timeout = ();
    type Message = ();

    #[allow(unused_variables)]
    fn ready(&mut self, event_loop: &mut EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        match token {           
            UDP_SERVER_TOKEN => { 
                let is_reregister = accept_udp_connection(self, event_loop);
                if is_reregister {
                    reregister(event_loop, &self.udp_server, token);
                }
            },
            _ => {
                let rx_conn = self.rx_connections.get_mut(token);
                if rx_conn.is_some() {
                    rx_conn.unwrap().socket_ready(event_loop, token, events);
                    return;
                }
                let tx_conn = self.tx_connections.get_mut(token);
                if tx_conn.is_some() {
                    tx_conn.unwrap().socket_ready(event_loop, token, events, &self.udp_server);
                    return;
                }
                panic!("Unknown token. Memory leak");
           }
        }
    }
}

fn reregister(event_loop: &mut EventLoop<MioServer>, evented: &mio::Evented, token: Token) {
    let _ = event_loop.reregister(evented, token, EventSet::readable(), PollOpt::edge() | PollOpt::oneshot());
}


fn accept_udp_connection(mio_server: &mut MioServer, event_loop: &mut EventLoop<MioServer>) -> bool {
    println!("the server socket is ready to accept a UDP connection");
    //note: sampel echo server uses MutSliceBuf with a pre-allocated size. Would be faster,
    //      but it's awkward to handle except for writing to sockets (how to convert to string for debugging?)
    
    //todo: guaranteed to read the whole packet?
    let mut buf = Vec::<u8>::new();
    match mio_server.udp_server.recv_from(&mut buf) {
        Ok(Some(addr)) => {            
            println!("received data from {:?}", addr);

            let rx_token = mio_server.rx_connections
                            .insert_with(|rx_token| InboundConnection::new(rx_token, addr))
                            .unwrap();


            println!("Processing as txn: {:?}", rx_token);
            let tx = UdpSocket::v4().unwrap();            
            let tx_token = mio_server.tx_connections
                                .insert_with(|tx_token| OutboundConnection::new(tx_token, tx, buf, rx_token, addr))
                                .unwrap();

            println!("Upstream token is {:?}", tx_token);
            let _ = event_loop.register_opt(&mio_server.tx_connections[tx_token].tx_socket, tx_token, EventSet::writable(), PollOpt::edge() | PollOpt::oneshot());

            return true;
            
        }
        Ok(None) => println!("The udp socket wasn't actually ready"),
        Err(e) => println!("couldn't receive a datagram: {}", e)
    }    
    //todo: get guidance from carllerche on when you need to reregister
    return false;
}

fn start(udp_server: mio::udp::UdpSocket) {

    let mut event_loop = mio::EventLoop::new().unwrap();
    let _ = event_loop.register_opt(&udp_server, UDP_SERVER_TOKEN, mio::EventSet::readable(), mio::PollOpt::edge() | mio::PollOpt::oneshot());

    println!("running mio server");
    //todo: strategy for number of connections.
    let mut mio_server = MioServer {
        udp_server: udp_server,
        rx_connections: Slab::new_starting_at(mio::Token(UDP_SERVER_TOKEN.as_usize()), 1024),
        tx_connections: Slab::new_starting_at(mio::Token(1025), 2048)
    };
    let _ = event_loop.run(&mut mio_server);
    
    drop(mio_server.udp_server);
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
    
   
