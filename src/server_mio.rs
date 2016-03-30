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
    //cache: Arc<RwLock<Cache>>
}

impl Handler for MioServer {
    type Timeout = Token;
    type Message = String; //todo: make enum

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        let mut ctx = RequestCtx::new(event_loop, events, token);
        debug!("MioServer.ready() {:?}", ctx.events);
        match token {
            UdpServer::UDP_SERVER_TOKEN => self.udp_server.server_ready(&mut ctx),
            TcpServer::TCP_SERVER_TOKEN => self.tcp_server.server_ready(&mut ctx),
            udp_tok if self.udp_server.base.owns(udp_tok) => self.udp_server.request_ready(&mut ctx),
            tcp_tok if self.tcp_server.base.owns(tcp_tok) => self.tcp_server.request_ready(&mut ctx),
            unknown_tok => error!("Unknown token {:?}", unknown_tok)
        }
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, event_loop: &mut EventLoop<Self>, token: Self::Timeout) {
        info!("Got timeout: {:?}", token);
        let mut ctx = RequestCtx::new(event_loop, EventSet::none(), token);
        match token {
            udp_tok if self.udp_server.base.owns(udp_tok) => self.udp_server.base.timeout(&mut ctx),
            tcp_tok if self.tcp_server.base.owns(tcp_tok) => self.tcp_server.base.timeout(&mut ctx),
            unknown_tok => error!("Unknown token {:?}", unknown_tok)
        }
        self.ready(ctx.event_loop, token, EventSet::none());
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: String) {
        //TODO: finish
        info!("Got a message {}", msg);
        if msg == format!("{}", "Stop!") {
            event_loop.shutdown()
        }
    }
}

pub struct RequestCtx<'a> {
    pub event_loop: &'a mut EventLoop<MioServer>,
    pub events: EventSet,
    pub token: Token,
}

impl<'a> RequestCtx<'a> {
    pub fn new(event_loop: &mut EventLoop<MioServer>,
            events: EventSet,
            token: Token)
            -> RequestCtx {
        return RequestCtx {
            event_loop: event_loop,
            events: events,
            token: token,
        };
    }
}

impl MioServer {

    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Option<Sender<String>>, JoinHandle<()>) {

        
        let run_handle = thread::Builder::new()
                             .name("dns_srv_net_io".to_string())
                             .spawn(move || {

                                let mut event_loop = EventLoop::<MioServer>::new().unwrap();
                                //let tx = event_loop.channel();
                                let max_connections = u16::max_value() as usize;
                                let start_token = 2;

                                let params = RequestParams {
                                    timeout: timeout,
                                    upstream_addr: upstream_server,
                                };

                                let cache = Cache::new();
                                let shared_cache = Arc::new(RwLock::new(cache));

                                let udp_server = UdpServer::new(address, start_token, max_connections, params, shared_cache.clone());
                                let tcp_server = TcpServer::new(address, start_token, max_connections, params, shared_cache.clone());

                                //TODO: event loop per core?
                                
                                let _ = event_loop.register(&udp_server.server_socket,
                                                            UdpServer::UDP_SERVER_TOKEN,
                                                            EventSet::readable(),
                                                            PollOpt::edge() | PollOpt::oneshot());

                               let _ = event_loop.register(&tcp_server.server_socket,
                                                            TcpServer::TCP_SERVER_TOKEN,
                                                            EventSet::readable(),
                                                            PollOpt::edge() | PollOpt::oneshot());
                            
                                let mut mio_server = MioServer {
                                    udp_server: udp_server,
                                    tcp_server: tcp_server,
                                };
                                 info!("Mio server running...");
                                 let _ = event_loop.run(&mut mio_server);
                                drop(mio_server.udp_server);
                                drop(mio_server.tcp_server);
                             })
                             .unwrap_or_else(|e| {
                                 panic!("Failed to start server thread. Error was {}", e)
                             });
        return (None, run_handle);
    }
}
