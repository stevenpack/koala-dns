use std::thread::JoinHandle;
use std::net::SocketAddr;
use server_mio::MioServer;
use mio::Sender;

///
/// Rust DNS server.
/// 
/// Server binds TCP and UDP sockets using Mio. As requests come in, they are routed through the ReqeustPipeline
/// New requests are parsed, checked for an authoritive answer, a cached answer, or are forwarded upstream.
/// All socket events are either incoming requests, or read/writes from the Upstream requests. Responses are
/// written immediately
///
pub struct Server {
    pub port: u32,
    pub upstream_server: SocketAddr,
    pub timeout: u64,
    pub master_file: String,
    pub sender: Option<Sender<String>>,
}

pub trait ServerOps {
    fn new(port: u32, upstream_server: SocketAddr, timeout: u64, master_file: String) -> Server;
    fn start(&mut self);
    fn begin_start(&mut self) -> JoinHandle<()>;
    fn stop(&mut self);
}

impl ServerOps for Server {
    fn new(port: u32, upstream_server: SocketAddr, timeout: u64, master_file: String) -> Server {
        Server {
            port: port,
            upstream_server: upstream_server,
            timeout: timeout,
            master_file: master_file,
            sender: None,
        }        
    }

    fn start(&mut self) {
        loop {
            let run_handle = self.begin_start();
            let result = run_handle.join();
            error!("Server thread returned. Reason: {:?} Restarting...", result);
        }
    }

    fn begin_start(&mut self) -> JoinHandle<()> {
        info!("Starting server on port {} and upstream {}",
              self.port,
              self.upstream_server);
        let address_str = format!("0.0.0.0:{:?}", self.port);
        let address = address_str.parse().unwrap_or_else(|e| panic!("Couldn't parse address {:?} {:?}", address_str, e));
        let (tx, run_handle) = MioServer::start(address, self.upstream_server, self.timeout);
        self.sender = Some(tx);
        info!("Joining on run handle");
        run_handle
    }

    fn stop(&mut self) {
        match self.sender {
            Some(ref x) => x.send(format!("{:?}", "Stop!")).unwrap(),
            None => warn!("Sender is null. Have you called start?"),
        };
    }
}
