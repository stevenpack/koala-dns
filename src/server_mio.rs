extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use std::net::SocketAddr;
use std::thread;
use std::thread::JoinHandle;
use std::sync::{Arc, RwLock};
use mio::Sender;
use request::base::{RequestParams};
use cache::*;
use servers::udp::UdpServer;
use servers::tcp::TcpServer;

pub struct MioServer {
    udp_server: UdpServer,
    tcp_server: TcpServer,
    cache: SharedCache
}

impl Handler for MioServer {
    type Timeout = Token;
    type Message = String;

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        //TODO: Expensive to clone the cache ref for every request? Can be stored on ServerBase. It's on
        //RequestCtx for convenience so pipeline stages can just take the ctx as a parm and have everything
        let mut ctx = RequestCtx::new(event_loop, events, token, self.cache.clone());
        debug!("MioServer.ready() {:?}", ctx.events);
        match token {
            UdpServer::UDP_SERVER_TOKEN => self.udp_server.server_ready(&mut ctx),
            TcpServer::TCP_SERVER_TOKEN => self.tcp_server.server_ready(&mut ctx),
            udp_tok if self.udp_server.owns(udp_tok) => self.udp_server.request_ready(&mut ctx),
            tcp_tok if self.tcp_server.owns(tcp_tok) => self.tcp_server.request_ready(&mut ctx),
            unknown_tok => error!("Unknown token {:?}", unknown_tok)
        }
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, event_loop: &mut EventLoop<Self>, token: Self::Timeout) {
        info!("Got timeout: {:?}", token);
        let mut ctx = RequestCtx::new(event_loop, EventSet::none(), token, self.cache.clone());
        match token {
            udp_tok if self.udp_server.base.owns(udp_tok) => self.udp_server.base.timeout(&mut ctx),
            tcp_tok if self.tcp_server.base.owns(tcp_tok) => self.tcp_server.base.timeout(&mut ctx),
            unknown_tok => error!("Unknown token {:?}", unknown_tok)
        }
        self.ready(ctx.event_loop, token, EventSet::none());
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: String) {
        //For tests. Could implement SIG handling
        info!("Got a message {}", msg);
        if msg == "Stop!" {
            event_loop.shutdown()
        }
    }
}

pub type SharedCache = Arc<RwLock<Cache>>;
pub struct RequestCtx<'a> {
    pub event_loop: &'a mut EventLoop<MioServer>,
    pub events: EventSet,
    pub token: Token,
    pub cache: SharedCache
}

impl<'a> RequestCtx<'a> {
    pub fn new(event_loop: &mut EventLoop<MioServer>,
            events: EventSet,
            token: Token,
            cache: SharedCache)
            -> RequestCtx {
        RequestCtx {
            event_loop: event_loop,
            events: events,
            token: token,
            cache: cache
        }
    }
}

impl MioServer {

    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Sender<String>, JoinHandle<()>) {

        let mut event_loop = EventLoop::<MioServer>::new().unwrap();        
        let sender = event_loop.channel();
        let run_handle = thread::Builder::new()
            .name("dns_srv_net_io".to_string())
            .spawn(move || {
                
                let max_connections = u16::max_value() as usize;

                let params = RequestParams {
                    timeout: timeout,
                    upstream_addr: upstream_server,
                };

                let udp_server = UdpServer::new(address, max_connections, params);
                let tcp_server = TcpServer::new(address, max_connections, params);

                //TODO: event loop per core?

                let _ = event_loop.register(&udp_server.server_socket,
                                            UdpServer::UDP_SERVER_TOKEN,
                                            EventSet::readable(),
                                            PollOpt::edge() | PollOpt::oneshot());

                let _ = event_loop.register(&tcp_server.server_socket,
                                            TcpServer::TCP_SERVER_TOKEN,
                                            EventSet::readable(),
                                            PollOpt::edge() | PollOpt::oneshot());

                let cache = Cache::default();
                let mut mio_server = MioServer {
                    udp_server: udp_server,
                    tcp_server: tcp_server,
                    cache: Arc::new(RwLock::new(cache))
                };
                info!("Start server...");
                let result = event_loop.run(&mut mio_server);
                info!("{:?}", result);
                drop(mio_server.udp_server);
                drop(mio_server.tcp_server);
            })
            .unwrap_or_else(|e| {
             panic!("Failed to start server thread. Error was {}", e)
            });
        (sender, run_handle)
    }
}
