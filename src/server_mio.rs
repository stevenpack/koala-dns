extern crate mio;
extern crate bytes;

use mio::{Evented, Token, EventLoop, EventSet, PollOpt, Handler};
use mio::udp::UdpSocket;
use mio::tcp::{TcpListener, TcpStream};
use mio::util::Slab;
use std::net::SocketAddr;
use std::thread;
use std::thread::JoinHandle;
use std::io::{Read, Write};
use mio::{Sender, TryRead};
use udp_request::UdpRequest;
use dns::dns_entities::*;
use std::collections::HashMap;

pub struct MioServer {
    udp_server: UdpSocket,
    tcp_server: TcpListener,
    upstream_server: SocketAddr,
    timeout: u64,
    pending: HashMap<Token, TcpStream>, // tcp
    requests: Slab<UdpRequest>,
    responses: Vec<UdpRequest>,
}

const TCP_SERVER_TOKEN: Token = Token(1);
const UDP_SERVER_TOKEN: Token = Token(2);

impl Handler for MioServer {
    type Timeout = Token;
    type Message = String; //todo: make enum

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        debug!("TOKEN: {:?}", token);
        match token {
            TCP_SERVER_TOKEN => self.server_ready(event_loop, events, token),
            UDP_SERVER_TOKEN => self.server_ready(event_loop, events, token),
            request_token if self.pending.contains_key(&token) => {
                info!("TCP pending conn");
                match self.pending.get_mut(&token) {
                    Some(mut stream) => {
                        Self::receive_tcp(&mut stream, token, event_loop);
                    }
                    _ => {}
                }

            }
            request_token => self.request_ready(event_loop, events, request_token),
            // pending_tcp_request_token
            // request_token if pending_tcp_conn => ...accept tcp,
            // request_token if not => self.request_ready...
        }
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
    fn server_ready(&mut self,
                    event_loop: &mut EventLoop<MioServer>,
                    events: EventSet,
                    token: Token) {
        if events.is_readable() {
            match token {
                TCP_SERVER_TOKEN => {
                    self.accept_tcp(event_loop, events);
                    self.reregister_tcp_server(event_loop, EventSet::readable());
                }
                UDP_SERVER_TOKEN => {
                    self.accept_udp(event_loop, events);
                    self.reregister_udp_server(event_loop, EventSet::readable());
                }
                _ => error!("Unexpected server token: {:?}", token),
            }

        }
        if events.is_writable() {
            self.send_reply();
        }
        // We are always listening for new requests. The server socket will be regregistered
        // as writable if there are responses to write


        // todo: check events.remove() and add() as way to go writable...
    }

    fn request_ready(&mut self,
                     event_loop: &mut EventLoop<MioServer>,
                     events: EventSet,
                     token: Token) {

        let mut queue_response = false;
        match self.requests.get_mut(token) {
            Some(mut request) => {
                request.ready(event_loop, token, events);
                queue_response = request.has_reply();
            }
            None => warn!("{:?} not in requests", token),
        }
        if queue_response {
            self.queue_response(token);
            self.reregister_udp_server(event_loop, EventSet::readable() | EventSet::writable());
            self.reregister_tcp_server(event_loop, EventSet::readable() | EventSet::writable());
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

    fn reregister_udp_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        debug!("Re-registered: {:?} with {:?}", UDP_SERVER_TOKEN, events);
        let _ = event_loop.reregister(&self.udp_server,
                                      UDP_SERVER_TOKEN,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
    }

    fn reregister_tcp_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        debug!("Re-registered: {:?} with {:?}", TCP_SERVER_TOKEN, events);
        let _ = event_loop.reregister(&self.tcp_server,
                                      TCP_SERVER_TOKEN,
                                      events,
                                      PollOpt::edge() | PollOpt::oneshot());
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

    fn add_transaction(&mut self, addr: SocketAddr, bytes: &[u8]) -> Option<Token> {
        let upstream_server = self.upstream_server;
        let timeout_ms = self.timeout;
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        return self.requests
                   .insert_with(|tok| UdpRequest::new(tok, addr, upstream_server, buf, timeout_ms));
    }

    fn accept_tcp(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        match self.tcp_server.accept() {
            Ok(Some((stream, addr))) => {
                // match TcpStream::connect(&addr) {
                //     Ok(mut connected_stream) => {

                match self.add_transaction(addr, Vec::<u8>::new().as_slice()) {
                    Some(tok) => {
                        let reg = event_loop.register(&stream,
                                                      tok,
                                                      EventSet::readable(),
                                                      PollOpt::edge() | PollOpt::oneshot());
                        info!("Registered tcp socket for read {:?} {:?} {:?}",
                              stream,
                              tok,
                              reg);

                        self.pending.insert(tok, stream);

                    }
                    None => error!("add_transaction failed"),
                }
                //     }
                //     Err(err) => error!("{}", err),
                // }
            }
            Ok(None) => debug!("Socket would block waiting..."),
            Err(err) => error!("Failed to accept tcp connection {}", err),
        }
    }

    fn accept_udp(&mut self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        let new_tok = self.receive_udp(&self.udp_server)
                          .and_then(|(addr, buf)| self.add_transaction(addr, buf.as_slice()));

        if new_tok.is_some() {
            debug!("There are {:?} in-flight requests", self.requests.count());
            self.ready(event_loop, new_tok.unwrap(), events);
        } else {
            error!("Failed to add request. New Token was None");
        }
    }

    fn receive_tcp(stream: &mut TcpStream, token: Token, event_loop: &mut EventLoop<MioServer>) {
        info!("Have a TcpStream to receive from to {:?}", stream);

        let mut buf = Vec::<u8>::with_capacity(512);
        match stream.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                info!("Read 0 bytes");
                // let reg = event_loop.register(stream,
                //                               token,
                //                               EventSet::readable(),
                //                               PollOpt::edge() | PollOpt::oneshot());


            }
            Ok(Some(n)) => {
                info!("Read {} bytesxx", n);
                buf.truncate(n);
            }
            Ok(None) => info!("None"),
            Err(err) => error!("read failed {}", err),

        }
        info!("Read {} bytes", buf.len());
        // TODO: FIRST TWO BYTES IN TCP ARE LENGTH
        let mut b2 = Vec::from(buf);
        let b3 = b2.split_off(2);
        let msg = DnsMessage::parse(&b3);
        debug!("{:?}", msg);

        // let mut buf = Vec::with_capacity(512);
        // // I think we're meant ot register the TcpStream or UdpSocket as evented... there
        // // might be lots of reads.
        // match stream.read_to_end(&mut buf) {
        //     Ok(cnt) => {
        //         buf.truncate(cnt);
        //         let msg = DnsMessage::parse(&buf);
        //         info!("Got: {:?}", msg);
        //     }
        //     Err(err) => error!("Failed to read {}", err),
        // }
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

    fn register_server(event_loop: &mut EventLoop<MioServer>,
                       server_socket: &Evented,
                       token: Token) {
        let reg = event_loop.register(server_socket,
                                      token,
                                      EventSet::readable(),
                                      PollOpt::edge() | PollOpt::oneshot());

        info!("server registration {:?}", reg);
    }

    pub fn start(address: SocketAddr,
                 upstream_server: SocketAddr,
                 timeout: u64)
                 -> (Sender<String>, JoinHandle<()>) {


        let udp_server = Self::bind_udp(address);
        let tcp_server = Self::bind_tcp(address);

        let mut event_loop = EventLoop::new().unwrap();

        Self::register_server(&mut event_loop, &udp_server, UDP_SERVER_TOKEN);
        Self::register_server(&mut event_loop, &tcp_server, TCP_SERVER_TOKEN);

        let tx = event_loop.channel();
        let max_connections = u16::max_value() as usize;
        let mut mio_server = MioServer {
            udp_server: udp_server,
            tcp_server: tcp_server,
            upstream_server: upstream_server,
            timeout: timeout,
            pending: HashMap::<Token, TcpStream>::new(),
            requests: Slab::new_starting_at(Token(3), max_connections),
            responses: Vec::<UdpRequest>::new(),
        };
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
