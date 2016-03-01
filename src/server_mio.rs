extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::udp::UdpSocket;
use mio::tcp::TcpListener;
use mio::util::Slab;
use std::net::SocketAddr;
use std::thread;
use std::thread::JoinHandle;
use mio::Sender;
use request::udp_request::UdpRequest;
use request::request_base::{RequestBase, RequestParams};
use servers::udp::UdpServer;
use servers::tcp::TcpServer;

pub struct MioServer {
    udp_server: UdpServer,
    tcp_server: TcpListener,
    upstream_server: SocketAddr,
    timeout: u64,
    responses: Vec<UdpRequest>,
}

const UDP_SERVER_TOKEN: Token = Token(1);
const TCP_SERVER_TOKEN: Token = Token(0);

impl Handler for MioServer {
    type Timeout = Token;
    type Message = String; //todo: make enum

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {

        let mut ctx = RequestContext::from(event_loop, events, token);

        match token {
            UDP_SERVER_TOKEN => self.server_ready(ctx),
            TCP_SERVER_TOKEN => debug!("TCP CONNECT"),
            _ => self.request_ready(&mut ctx),
        }
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, event_loop: &mut EventLoop<Self>, token: Self::Timeout) {
        info!("Got timeout: {:?}", token);
        let mut ctx = RequestContext::from(event_loop, EventSet::none(), token);
        match self.udp_server.requests.get_mut(token) {
            Some(mut request) => request.inner.on_timeout(token),
            None => warn!("Timed out request wasn't present. {:?}", token),
        }
        self.request_ready(&mut ctx);
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
    fn from(event_loop: &mut EventLoop<MioServer>,
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
    fn server_ready(&mut self, ctx: RequestContext) {
        if ctx.events.is_readable() {
            let tok = self.udp_server.accept(&ctx);
            if tok.is_some() {
                self.ready(ctx.event_loop, tok.unwrap(), ctx.events);
            }
        }
        if ctx.events.is_writable() {
            self.send_reply();
        }
        // We are always listening for new requests. The server socket will be regregistered
        // as writable if there are responses to write
        self.reregister_server(ctx.event_loop, EventSet::readable());
    }

    fn request_ready(&mut self, ctx: &mut RequestContext) {

        let mut queue_response = false;
        match self.udp_server.requests.get_mut(ctx.token) {
            Some(mut request) => {
                request.ready(ctx);
                queue_response = request.inner.has_reply();
            }
            None => warn!("{:?} not in requests", ctx.token),
        }
        if queue_response {
            self.queue_response(ctx.token);
            self.reregister_server(ctx.event_loop, EventSet::readable() | EventSet::writable());
        }
    }

    fn queue_response(&mut self, token: Token) {
        match self.remove_request(token) {
            Some(request) => {
                self.responses.push(request);
                debug!("Added {:?} to pending replies", token);
            }
            None => error!("Failed to remove request and queue response. {:?}", token),
        }
    }

    fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        debug!("Re-registered: {:?} with {:?}", UDP_SERVER_TOKEN, events);
        let _ = event_loop.reregister(&self.udp_server.server_socket,
                                      UDP_SERVER_TOKEN,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
    }

    fn remove_request(&mut self, token: Token) -> Option<UdpRequest> {
        match self.udp_server.requests.remove(token) {
            Some(request) => {
                debug!("Removed {:?} from pending requests.", token);
                return Some(request);
            }
            None => warn!("No request found {:?}", token),
        }
        return None;
    }

    fn send_reply(&mut self) {
        debug!("There are {} responses to send", self.responses.len());
        match self.responses.pop() {
            Some(reply) => reply.send(&self.udp_server.server_socket),
            None => warn!("Nothing to send."),
        }
    }





    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Sender<String>, JoinHandle<()>) {

        let max_connections = u16::max_value() as usize;

        let udp_socket = UdpServer::bind_udp(address);
        let params = RequestParams {
            timeout: timeout,
            upstream_addr: upstream_server,
        };

        let udp_server = UdpServer::new(udp_socket, Slab::new_starting_at(Token(2), max_connections),params );
        let tcp_server = TcpServer::bind_tcp(address);

        let mut event_loop = EventLoop::new().unwrap();
        let _ = event_loop.register(&udp_server.server_socket,
                                    UDP_SERVER_TOKEN,
                                    EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot());

        let _ = event_loop.register(&tcp_server,
                                    TCP_SERVER_TOKEN,
                                    EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot());
        let tx = event_loop.channel();

        let mut mio_server = MioServer {
            udp_server: udp_server,
            tcp_server: tcp_server,
            upstream_server: upstream_server,
            timeout: timeout,
            responses: Vec::<UdpRequest>::new(),
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
