extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use std::net::SocketAddr;
use std::thread;
use std::thread::JoinHandle;
use mio::Sender;
use request::base::{RequestParams};
use servers::udp::UdpServer;
use servers::tcp::TcpServer;

pub struct MioServer {
    udp_server: UdpServer,
    tcp_server: TcpServer,
    // upstream_server: SocketAddr,
    // timeout: u64,
}

impl Handler for MioServer {
    type Timeout = Token;
    type Message = String; //todo: make enum

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {

        let mut ctx = RequestContext::new(event_loop, events, token);

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
        let mut ctx = RequestContext::new(event_loop, EventSet::none(), token);
        match token {
            udp_tok if self.udp_server.base.owns(udp_tok) => self.udp_server.base.timeout(&mut ctx),
            tcp_tok if self.tcp_server.base.owns(tcp_tok) => self.tcp_server.base.timeout(&mut ctx),
            unknown_tok => error!("Unknown token {:?}", unknown_tok)
        }
        self.ready(ctx.event_loop, token, EventSet::none());
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: String) {
        // todo: finish
        info!("Got a message {}", msg);
        if msg == format!("{}", "Stop!") {
            event_loop.shutdown()
        }
    }
}

pub struct RequestContext<'a> {
    pub event_loop: &'a mut EventLoop<MioServer>,
    pub events: EventSet,
    pub token: Token,
}

impl<'a> RequestContext<'a> {
    pub fn new(event_loop: &mut EventLoop<MioServer>,
            events: EventSet,
            token: Token)
            -> RequestContext {
        return RequestContext {
            event_loop: event_loop,
            events: events,
            token: token,
        };
    }
}

impl MioServer {

    pub fn reregister_server(event_loop: &mut EventLoop<MioServer>, events: EventSet, token: Token, socket: &Evented) {
        debug!("Re-registered: {:?} with {:?}", token, events);
        let _ = event_loop.reregister(socket,
                                      token,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
    }

    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Sender<String>, JoinHandle<()>) {

        let max_connections = u16::max_value() as usize;
        let start_token = 2;

        let params = RequestParams {
            timeout: timeout,
            upstream_addr: upstream_server,
        };

        let udp_server = UdpServer::new(address, start_token, max_connections, params);
        let tcp_server = TcpServer::new(address, start_token, max_connections, params);

        let mut event_loop = EventLoop::new().unwrap();
        let _ = event_loop.register(&udp_server.server_socket,
                                    UdpServer::UDP_SERVER_TOKEN,
                                    EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot());

        let _ = event_loop.register(&tcp_server.server_socket,
                                    TcpServer::TCP_SERVER_TOKEN,
                                    EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot());
        let tx = event_loop.channel();

        let mut mio_server = MioServer {
            udp_server: udp_server,
            tcp_server: tcp_server
        };
        let run_handle = thread::Builder::new()
                             .name("dns_srv_net_io".to_string())
                             .spawn(move || {
                                 info!("Mio server running...");
                                 let _ = event_loop.run(&mut mio_server);
                                 drop(mio_server.udp_server);
                             })
                             .unwrap_or_else(|e| {
                                 panic!("Failed to start udp server. Error was {}", e)
                             });
        return (tx, run_handle);
    }
}
