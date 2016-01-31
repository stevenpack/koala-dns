extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::udp::UdpSocket;
use mio::tcp::{TcpListener, TcpStream};
use mio::util::Slab;
use std::net::SocketAddr;
use std::thread;
use std::thread::JoinHandle;
use mio::{Sender, TryRead};
use udp_request::UdpRequest;
use dns::dns_entities::*;
use std::collections::HashMap;
use socket::*;

pub struct MioServer {
    udp_server: UdpSocket,
    tcp_server: TcpListener,
    upstream_server: SocketAddr,
    timeout: u64,
    pending: HashMap<Token, TcpStream>,
    accepted: HashMap<Token, TcpStream>,
    requests: Slab<UdpRequest>,
    responses: Vec<UdpRequest>,
}

const TCP_SERVER_TOKEN: Token = Token(1);
const UDP_SERVER_TOKEN: Token = Token(2);
impl Handler for MioServer {
    type Timeout = Token;
    type Message = String; //todo: make enum

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        debug!("enter ready {:?} {:?}", token, events);
        match token {
            TCP_SERVER_TOKEN => self.server_ready(event_loop, events, token),
            UDP_SERVER_TOKEN => self.server_ready(event_loop, events, token),
            pending if self.pending.contains_key(&token) => {
                self.accept_pending(event_loop, events, pending)
            }
            request => self.request_ready(event_loop, events, request),
        }
        debug!("exit ready {:?} {:?}", token, events);
    }

    #[allow(unused_variables)]
    fn timeout(&mut self, event_loop: &mut EventLoop<Self>, token: Self::Timeout) {
        info!("Got timeout: {:?}", token);
        match self.requests.get_mut(token) {
            Some(mut request) => request.on_timeout(token),
            None => warn!("Timed out request wasn't present. {:?}", token),
        }
        self.request_ready(event_loop, EventSet::none(), token);
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: String) {
        // todo: finish
        info!("Got a message {}", msg);
        if msg == format!("{}", "Stop!") {
            event_loop.shutdown()
        }
    }
}

impl MioServer {
    fn server_ready(&mut self, event_loop: &mut EventLoop<Self>, events: EventSet, token: Token) {
        debug!("server_ready {:?} {:?}", token, events);
        if events.is_writable() {
            self.send_reply();
            return; //TODO: consider: keeps writing out requests before accepting new responses.
        }

        if events.is_readable() {
            match token {
                TCP_SERVER_TOKEN => self.accept_tcp(event_loop),
                UDP_SERVER_TOKEN => self.accept_udp(event_loop, events),
                _ => {
                    error!("Unexpected server token: {:?}", token);
                    return;
                }
            }
        }
        let mut events = EventSet::readable();
        if self.responses.len() > 0 {
            events = events | EventSet::writable();
        }
        self.reregister_server(event_loop, events, token);
    }

    fn accept_pending(&mut self,
                      event_loop: &mut EventLoop<Self>,
                      events: EventSet,
                      token: Token) {
        debug_assert!(events.is_readable());
        // TODO: We need keep a reference to the TcpStream to reply to
        match self.pending.remove(&token) {
            Some(mut stream) => {
                let buf = Self::receive_tcp(&mut stream);
                self.accepted.insert(token, stream);
                match self.requests.get_mut(token) {
                    Some(request) => {
                        request.query_buf = buf;
                    }
                    None => error!("Request {:?} not found", token),
                }
            }
            None => error!("{:?} was not pending", token),
        }
        self.ready(event_loop, token, events);
    }

    fn request_ready(&mut self, event_loop: &mut EventLoop<Self>, events: EventSet, token: Token) {
        debug!("request_ready {:?} {:?}", token, events);

        let mut queue_response = false;
        let mut server_token = UDP_SERVER_TOKEN;
        match self.requests.get_mut(token) {
            Some(mut request) => {
                request.ready(event_loop, token, events);
                queue_response = request.has_reply();
                server_token = request.server_token;
            }
            None => warn!("{:?} not in requests", token),
        }
        if queue_response {
            self.queue_response(token);
            self.reregister_server(event_loop,
                                   EventSet::readable() | EventSet::writable(),
                                   server_token);
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

    fn remove_request(&mut self, token: Token) -> Option<UdpRequest> {
        match self.requests.remove(token) {
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
            Some(reply) => reply.send(&self.udp_server),
            None => warn!("Nothing to send."),
        }
    }

    fn add_transaction(&mut self,
                       addr: SocketAddr,
                       bytes: &[u8],
                       server_token: Token)
                       -> Option<Token> {
        let upstream_server = self.upstream_server;
        let timeout_ms = self.timeout;
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        return self.requests
                   .insert_with(|tok| {
                       UdpRequest::new(tok, server_token, addr, upstream_server, buf, timeout_ms)
                   });
    }

    fn accept_tcp(&mut self, event_loop: &mut EventLoop<Self>) {
        match self.tcp_server.accept() {
            Ok(Some((stream, addr))) => {
                match self.add_transaction(addr, Vec::<u8>::new().as_slice(), TCP_SERVER_TOKEN) {
                    Some(tok) => {
                        Self::register(event_loop, &stream, EventSet::readable(), tok, true);
                        self.pending.insert(tok, stream);
                    }
                    None => error!("add_transaction failed"),
                }
            }
            Ok(None) => debug!("Socket would block waiting..."),
            Err(err) => error!("Failed to accept tcp connection {}", err),
        }
    }

    fn accept_udp(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        let new_tok = self.receive_udp(&self.udp_server)
                          .and_then(|(addr, buf)| {
                              self.add_transaction(addr, buf.as_slice(), UDP_SERVER_TOKEN)
                          });

        if new_tok.is_some() {
            debug!("There are {:?} in-flight requests", self.requests.count());
            self.ready(event_loop, new_tok.unwrap(), events);
        } else {
            error!("Failed to add request. New Token was None");
        }
    }

    fn receive_tcp(stream: &mut TcpStream) -> Vec<u8> {
        info!("Have a TcpStream to receive from to {:?}", stream);

        let mut buf = Vec::<u8>::with_capacity(512);
        match stream.try_read_buf(&mut buf) {
            Ok(Some(0)) => info!("Read 0 bytes"),
            Ok(Some(n)) => buf.truncate(n),
            Ok(None) => info!("None"),
            Err(err) => error!("read failed {}", err),

        }
        info!("Read {} bytes", buf.len());
        // TODO: FIRST TWO BYTES IN TCP ARE LENGTH
        let mut b2 = Vec::from(buf);
        let b3 = b2.split_off(2);
        let msg = DnsMessage::parse(&b3);
        debug!("{:?}", msg);
        b2
    }

    fn receive_udp(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        // 2.3.4 Size Limits from RFC1035
        let mut buf = vec![0;512];
        match socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {}", count, addr);
                // todo: Shoudl we actually re-register until we get Ok(None)?
                buf.truncate(count);
                let msg = DnsMessage::parse(&buf);
                debug!("{:?}", msg);
                return Some((addr, buf));
            }
            Ok(None) => {
                debug!("No bytes reeived on UDP socket");
                return None;
            }
            Err(e) => {
                error!("Receive failed {:?}", e);
                return None;
            }
        };
    }

    fn bind_udp(address: SocketAddr) -> UdpSocket {
        info!("Binding UDP to {:?}", address);
        let udp_socket = UdpSocket::v4().unwrap_or_else(|e| {
            panic!("Failed to create udp server socket {}", e)
        });
        let _ = udp_socket.bind(&address)
                          .unwrap_or_else(|e| panic!("Failed to bind udp socket. Error was {}", e));
        return udp_socket;
    }

    fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }

    // fn register_socket(event_loop: &mut EventLoop<MioServer>, socket: &Evented, token: Token) {
    //     Self::register
    // }

    fn register_server(&self, event_loop: &mut EventLoop<MioServer>, token: Token) {
        let read = EventSet::readable();
        match token {
            UDP_SERVER_TOKEN => Self::register(event_loop, &self.udp_server, read, token, false),
            TCP_SERVER_TOKEN => Self::register(event_loop, &self.tcp_server, read, token, false),
            _ => error!("{:?} is not a server socket token", token),
        }
    }

    fn reregister_server(&self,
                         event_loop: &mut EventLoop<MioServer>,
                         events: EventSet,
                         token: Token) {
        match token {
            UDP_SERVER_TOKEN => Self::register(event_loop, &self.udp_server, events, token, true),
            TCP_SERVER_TOKEN => Self::register(event_loop, &self.tcp_server, events, token, true),
            _ => error!("{:?} is not a server socket token", token),
        }
    }


    fn register(event_loop: &mut EventLoop<MioServer>,
                socket: &Evented,
                events: EventSet,
                token: Token,
                reregister: bool) {

        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        if reregister {
            let reg = event_loop.reregister(socket, token, events, poll_opt);
            debug!("Re-registered {:?} {:?} {:?}", token, events, reg);
        } else {
            let reg = event_loop.register(socket, token, events, poll_opt);
            debug!("Registered {:?} {:?} {:?}", token, events, reg);
        }
    }
    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Sender<String>, JoinHandle<()>) {


        let udp_server = Self::bind_udp(address);
        let tcp_server = Self::bind_tcp(address);

        let mut event_loop = EventLoop::new().unwrap();

        let tx = event_loop.channel();
        let max_connections = u16::max_value() as usize;
        let mut mio_server = MioServer {
            udp_server: udp_server,
            tcp_server: tcp_server,
            upstream_server: upstream_server,
            timeout: timeout,
            pending: HashMap::<Token, TcpStream>::new(),
            accepted: HashMap::<Token, TcpStream>::new(),
            requests: Slab::new_starting_at(Token(3), max_connections),
            responses: Vec::<UdpRequest>::new(),
        };

        mio_server.register_server(&mut event_loop, UDP_SERVER_TOKEN);
        mio_server.register_server(&mut event_loop, TCP_SERVER_TOKEN);


        let run_handle = thread::Builder::new()
                             .name("udp_srv_thread".to_string())
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
