extern crate mio;

use std::net::SocketAddr;
use std::io::Write;
use mio::{EventSet, PollOpt, Handler};
use mio::tcp::*;
use mio::udp::UdpSocket;
use mio::buf::*;

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
    udp_server: UdpSocket
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

fn bind_udp(address: SocketAddr) -> UdpSocket {
    println!("Binding UDP to {:?}", address);
    let udp_socket = UdpSocket::v4().unwrap();
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
            TCP_SERVER_TOKEN => { accept_tcp_connection(self); },
            UDP_SERVER_TOKEN => { accept_udp_connection(self); },
            _ => { panic!("Unknown token"); }
        }
    }
}

fn accept_tcp_connection(mio_server: &MioServer) {
        println!("the server socket is ready to accept a TCP connection");
        match mio_server.tcp_server.accept() {
            Ok(Some(mut connection)) => {
                println!("accepted a socket {}", connection.local_addr().unwrap());
                
                let quote = "What tcp bytes do you seek avatar?";
                let _ = connection.write_all(quote.as_bytes());
                drop(connection);
            }
            Ok(None) => {
                println!("the server socket wasn't actually ready");
            }
            Err(e) => {
                println!("listener.accept() errored: {}", e);
\            }
        }
}

fn accept_udp_connection(mio_server: &MioServer) {
        println!("the server socket is ready to accept a UDP connection");
        let mut buf = [0; 128];                
        match mio_server.udp_server.recv_from(&mut MutSliceBuf::wrap(&mut buf)) {
            Ok(Some(addr)) => {
                let quote = "What udp bytes do you seek avatar?";
                let mut quote_buf = SliceBuf::wrap(&mut quote.as_bytes());
                let _ = mio_server.udp_server.send_to(&mut quote_buf, &addr);
            }
            Ok(None) => println!("The udp socket wasn't actually ready"),
            Err(e) => println!("couldn't receive a datagram: {}", e)
        }
    }

fn start(tcp_server: TcpListener, udp_server: UdpSocket) {
    let mut event_loop = mio::EventLoop::new().unwrap();
    let _ = event_loop.register_opt(&tcp_server, TCP_SERVER_TOKEN, EventSet::readable(), PollOpt::edge());
    let _ = event_loop.register_opt(&udp_server, UDP_SERVER_TOKEN, EventSet::readable(), PollOpt::edge());

    println!("running mio server");
    let mut mio_server = MioServer {
        tcp_server: tcp_server,
        udp_server: udp_server
    };
    let _ = event_loop.run(&mut mio_server);
    
    drop(mio_server.udp_server);
    drop(mio_server.tcp_server);
}
   
    
   
