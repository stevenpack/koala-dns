use std::net::SocketAddr;
use server_mio::MioServer;

//
// Public shown to main
//
pub struct Server {
    pub port: u32,
    pub upstream_server: SocketAddr,
    pub timeout: u64
}

pub trait Start {
    fn start(&self);
}

impl Start for Server {
    fn start(&self) {
        info!("Starting server on port {} and upstream {}", self.port, self.upstream_server);
        let address = format!("0.0.0.0:{:?}", self.port).parse().unwrap();

        //todo: new thread, restart if die
        MioServer::start(address, self.upstream_server, self.timeout);
    }
}
