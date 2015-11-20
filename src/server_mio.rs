extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop};
use mio::tcp::*;
use std::net::SocketAddr;
use std::io::Write;
/*
Public shown to main
*/
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
    tcp_server: TcpListener,
    udp_server: mio::udp::UdpSocket
}

/*
A request reply transaction
*/
struct Transaction<T: Transport> {
    //InboundTransport (TcpSocket or UDP Address)
    //Outbound (TcpSocket or UDP Address)
    //Query (parsed and raw)
    //Response (parsed and raw)
    id: u32,
    rx: T,
    tx: T
}

struct UdpTransport  {
    addr: u32
}

trait Transport {
    fn receive(&self);    
    fn send(&self);
}

impl Transport for UdpTransport {
    fn receive(&self) {
        println!("receive UDP using my addr: {}", self.addr);
    }
    
    fn send(&self) {
        println!("send UDP");
    }
    
}

impl Start for Server {
    fn start(&self) {
        println!("Starting server on port {}", self.port);        
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();        
        let tcp_server = bind_tcp(address);
        let udp_server = bind_udp(address);
        start(tcp_server, udp_server);
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

fn bind_tcp(address: SocketAddr) -> TcpListener {
    println!("Binding TCP to {:?}", address);
    let tcp_server = match TcpListener::bind(&address) {
        Ok(s) => s,
        Err(e) => {
            panic!("Failed to bind TCP. Error was {}", e);
        }
    };
    return tcp_server;
}

const TCP_SERVER_TOKEN: mio::Token = mio::Token(0);
const UDP_SERVER_TOKEN: mio::Token = mio::Token(1);

impl mio::Handler for MioServer {
    type Timeout = ();
    type Message = ();

    #[allow(unused_variables)]
    fn ready(&mut self, event_loop: &mut mio::EventLoop<MioServer>, token: mio::Token, events: mio::EventSet) {
        match token {
            TCP_SERVER_TOKEN => { 
                let is_reregister = accept_tcp_connection(&self.tcp_server); 
                if is_reregister {
                    reregister(event_loop, &self.tcp_server, token);
                }
            },
            UDP_SERVER_TOKEN => { 
                let is_reregister = accept_udp_connection(&self.udp_server);
                if is_reregister {
                    reregister(event_loop, &self.udp_server, token);
                }
            },
            _ => { panic!("Unknown token"); }
        }
    }
}

fn reregister(event_loop: &mut EventLoop<MioServer>, evented: &mio::Evented, token: Token) {
    let _ = event_loop.reregister(evented, token, mio::EventSet::readable(), mio::PollOpt::edge() | mio::PollOpt::oneshot());
}

fn accept_tcp_connection(tcp_server: &TcpListener) -> bool {
    println!("the server socket is ready to accept a TCP connection");
    match tcp_server.accept() {
        Ok(Some(mut connection)) => {
            println!("accepted a tcp socket {}", connection.local_addr().unwrap());

            //TODO: add to list of connected clients
            //parse query
            //in cache?
            //yes, get from cache
            //no, get from upstream
                // register for writable
                // write

            let quote = "What tcp bytes do you seek avatar?";
            let _ = connection.write_all(quote.as_bytes());
            drop(connection);

            return true;
        }
        Ok(None) => {
            println!("the server socket wasn't actually ready");
        }
        Err(e) => {
            println!("listener.accept() errored: {}", e);
        }
    }
    return false;
}

fn accept_udp_connection(udp_server: &mio::udp::UdpSocket) -> bool {
    println!("the server socket is ready to accept a UDP connection");
    //note: sampel echo server uses MutSliceBuf with a pre-allocated size. Would be faster,
    //      but it's awkward to handle except for writing to sockets (how to convert to string for debugging?)
    let mut buf = Vec::<u8>::new();
    match udp_server.recv_from(&mut buf) {
        Ok(Some(addr)) => {

            //println!("remaining is {}", buf.remaining());
            match String::from_utf8(buf) {
                Ok(str) => {
                    println!("buffer is {}", str);        
                },
                Err(e) => {
                    println!("could't convert the buffer to utf8. {}", e);
                }
            }                      

            let quote = "What udp bytes do you seek avatar?";
            let mut quote_buf = mio::buf::SliceBuf::wrap(&mut quote.as_bytes());
            let _ = udp_server.send_to(&mut quote_buf, &addr);

            //todo: get guidance from carllerche on when you need to reregister
            return true;
            
        }
        Ok(None) => println!("The udp socket wasn't actually ready"),
        Err(e) => println!("couldn't receive a datagram: {}", e)
    }    
    return false;
}

fn start(tcp_server: TcpListener, udp_server: mio::udp::UdpSocket) {
    
    let t = Transaction::<UdpTransport> {
        id: 1,
        rx: UdpTransport{addr: 3},
        tx: UdpTransport{addr: 4}
    };
    t.rx.receive();
    t.tx.send();
    return;
    
    
    let mut event_loop = mio::EventLoop::new().unwrap();
    let _ = event_loop.register_opt(&tcp_server, TCP_SERVER_TOKEN, mio::EventSet::readable(), mio::PollOpt::edge() | mio::PollOpt::oneshot());
    let _ = event_loop.register_opt(&udp_server, UDP_SERVER_TOKEN, mio::EventSet::readable(), mio::PollOpt::edge() | mio::PollOpt::oneshot());

    println!("running mio server");
    let mut mio_server = MioServer {
        tcp_server: tcp_server,
        udp_server: udp_server
    };
    let _ = event_loop.run(&mut mio_server);
    
    drop(mio_server.udp_server);
    drop(mio_server.tcp_server);
}
   
    
   
