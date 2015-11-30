use std::net::SocketAddr;
use server_mio::MioServer;

//
// Public shown to main
//
// todo: get guidance from carllerche on when you need to reregister
pub struct Server {
    pub port: i32,
    pub upstream_server: SocketAddr
}

pub trait Start {
    fn start(&self);
}

impl Start for Server {
    fn start(&self) {
        info!("Starting server on port {} and upstream {}", self.port, self.upstream_server);
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();
        MioServer::start(address, self.upstream_server);
    }
}
