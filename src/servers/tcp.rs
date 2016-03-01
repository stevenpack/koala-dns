use std::net::SocketAddr;
use mio::tcp::TcpListener;

pub struct TcpServer;
impl TcpServer {
    pub fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }
}
