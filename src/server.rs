use std::thread::JoinHandle;
use std::net::SocketAddr;
use server_mio::MioServer;
use mio::Sender;

//
// Public shown to main
//
pub struct Server {
    pub port: u32,
    pub upstream_server: SocketAddr,
    pub timeout: u64,
    pub sender: Option<Sender<String>>,
}

pub trait ServerOps {
    fn new(port: u32, upstream_server: SocketAddr, timeout: u64) -> Server;
    fn start(&mut self);
    fn begin_start(&mut self) -> JoinHandle<()>;
    fn stop(&mut self);
}

impl ServerOps for Server {
    fn new(port: u32, upstream_server: SocketAddr, timeout: u64) -> Server {
        let server = Server {
            port: port,
            upstream_server: upstream_server,
            timeout: timeout,
            sender: None,
        };
        return server;
    }

    fn start(&mut self) {
        let run_handle = self.begin_start();
        let _ = run_handle.join();

        debug!("Thread returned. TODO: restart it!");
    }

    fn begin_start(&mut self) -> JoinHandle<()> {
        info!("Starting server on port {} and upstream {}",
              self.port,
              self.upstream_server);
        let address_str = format!("0.0.0.0:{:?}", self.port);
        let address = address_str.parse().unwrap_or_else(|e| panic!("Couldn't parse address {:?} {:?}", address_str, e));
        // TODO: new thread, restart if die
        let (tx, run_handle) = MioServer::start(address, self.upstream_server, self.timeout);
        self.sender = Some(tx);
        info!("Joining on run handle");
        return run_handle;
    }

    fn stop(&mut self) {
        match self.sender {
            Some(ref x) => x.send(format!("{:?}", "Stop!")).unwrap(),
            None => warn!("Sender is null. Have you called start?"),
        };
    }
}
