use std::net::SocketAddr;
use mio::Token;
use mio::tcp::TcpListener;
use server_mio::{MioServer,RequestContext};

pub struct TcpServer {
    pub server_socket: TcpListener
}
impl TcpServer {
    pub const TCP_SERVER_TOKEN: Token = Token(0);

    pub fn new(addr: SocketAddr) -> TcpServer {
        let listener = Self::bind_tcp(addr);
        TcpServer {
            server_socket: listener
        }
    }

    pub fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }

    fn request_ready(&mut self, ctx: &mut RequestContext) -> bool {
        false
    }
}
