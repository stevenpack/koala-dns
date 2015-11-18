use std::net::SocketAddr;
use mio::tcp::{TcpSocket,TcpListener};
use mio::udp::UdpSocket;

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
struct Sockets {
    tcp_server: TcpListener,
    udp_server: UdpSocket
}

impl Start for Server {
    fn start(&self) {
        println!("Starting server on port {}", self.port);        
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();        
        start(self, Sockets {
            tcp_server: bind_tcp(address),
            udp_server: bind_udp(address)
        });
    }    
}

fn bind_udp(address: SocketAddr) -> UdpSocket {
    println!("Binding UDP to {:?}", address);
    let udp_socket = UdpSocket::v4().unwrap();
    let udp_server = match udp_socket.bind(&address) {
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

 fn start(server: &Server, sockets: Sockets) {
        println!("hello");
        println!("{}", server.port);
}


    
   
    
   
