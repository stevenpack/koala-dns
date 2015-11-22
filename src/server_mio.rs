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
struct OutboundConnection {
    tx_token: Token,
    rx_token: Token,
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

    fn socket_ready(&self, token: Token, events: EventSet) {
        println!("I'm ready to write the client some data!!!");
    }
}

impl OutboundConnection {

    fn new(tx_token: Token, rx_token: Token) -> OutboundConnection {
        return OutboundConnection {
            tx_token: tx_token,
            rx_token: rx_token,
            response: None
        };
    }

    fn socket_ready(&self, token: Token, events: EventSet) {
        println!("I'm ready to write or read some data from upstream!");

        if events.is_writable() {
            println!("And the winner is... WRITING. Read token is: {:?}. Write token is: {:?}", self.rx_token, self.tx_token);
        } 

        if events.is_readable() {
            println!("And the winner is... READING");
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
                let rx_conn = self.rx_connections.get(token);
                if rx_conn.is_some() {
                    rx_conn.unwrap().socket_ready(token, events);
                    return;
                }
                let tx_conn = self.tx_connections.get(token);
                if tx_conn.is_some() {
                    tx_conn.unwrap().socket_ready(token, events);
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

            //let tx_addr = format!("8.8.8.8:{:?}", 53).parse().unwrap();
            let tx = UdpSocket::v4().unwrap();
            
            let tx_token = mio_server.tx_connections
                                .insert_with(|tx_token| OutboundConnection::new(tx_token, rx_token))
                                .unwrap();

            println!("Upstream token is {:?}", tx_token);
            let _ = event_loop.register_opt(&tx, tx_token, EventSet::writable(), PollOpt::edge() | PollOpt::oneshot());

            

            //event_loop.register(tx, writable, new token)
                //write the buffer
                    //event_loop.reregister(tx, readable, old token)
                    // read buffer
                        //event_loop.reregister(rx, writable, old token)
                            //write buffer.

            //let tx_listen_addr = format!("0.0.0.0:{:?}", 0).parse().unwrap();
            //tx.bind(&tx_listen_addr);
            // println!("Sending buf to {:?}", tx_addr);
            // let mut send_buf = mio::buf::SliceBuf::wrap(buf.as_slice());
            // match tx.send_to(&mut send_buf, &tx_addr) {
            //     Ok(Some(n)) => {println!("Sent some data: {:?} to: {:?}", n, tx_addr)},
            //     Ok(None) => {println!("Failed to send data")},
            //     Err(e) => {println!("Error sending {:?}", e)}
            // }

            // thread::sleep(Duration::new(2,0));

            // //request forwarded (with some id in it)
            // //now listen on...what...for response?
            // let mut response_buf = Vec::<u8>::new();
            // match tx.recv_from(&mut response_buf) {
            //     Ok(Some(add)) => {
            //         println!("Got a response from {:?}. Bytes were {:?}", add, response_buf.len() );
            //     },
            //     Ok(None) => { println!("Didn't receive anything")},
            //     Err(e) => println!("Error receiving {:?}", e)
 
            // }

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
    
   
