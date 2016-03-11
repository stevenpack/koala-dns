use std::net::SocketAddr;
use mio::{Token, EventSet};
use mio::util::Slab;
use mio::udp::UdpSocket;
use server_mio::{RequestCtx};
use request::base::*;
use request::udp::UdpRequest;
use servers::base::*;
use std::sync::{Arc, RwLock};

pub struct UdpServer {
    pub server_socket: UdpSocket,
    pub base: ServerBase<UdpRequest>
}

impl UdpServer {
    pub const UDP_SERVER_TOKEN: Token = Token(1);
    pub fn new(addr: SocketAddr, start_token: usize, max_connections: usize, params: RequestParams, cache: Arc<RwLock<Cache>>) -> UdpServer {
        let server_socket = Self::bind_udp(addr);
        let requests = Slab::new_starting_at(Token(start_token), max_connections);
        let responses = Vec::<UdpRequest>::new();
        UdpServer {
            server_socket: server_socket,
            base: ServerBase::<UdpRequest>::new(requests, responses, params, Self::UDP_SERVER_TOKEN, cache)
        }
    }

    pub fn bind_udp(address: SocketAddr) -> UdpSocket {
        info!("Binding UDP to {:?}", address);
        let udp_socket = UdpSocket::v4()
                             .unwrap_or_else(|e| panic!("Failed to create udp socket {}", e));
        let _ = udp_socket.bind(&address)
                          .unwrap_or_else(|e| panic!("Failed to bind udp socket. Error was {}", e));
        return udp_socket;
    }
    pub fn accept(&mut self, token: Token) -> Option<UdpRequest> {
        return self.receive(&self.server_socket)
            .and_then(|(addr, buf)| Some(self.base.build_request(token, addr, buf.as_slice())));
    }

    fn receive(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        // 2.3.4 Size Limits from RFC1035
        let mut buf = vec![0;512];
        match socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {}", count, addr);
                buf.truncate(count);
                return Some((addr, buf));
            }
            Ok(None) => {
                debug!("Server socket not ready to receive");
                return None;
            }
            Err(e) => {
                error!("Receive failed {:?}", e);
                return None;
            }
        };
    }

    pub fn server_ready(&mut self, ctx: &mut RequestCtx)  {
        if ctx.events.is_readable() {
            self.accept(ctx.token)
                .and_then( |req| self.base.requests.insert(req).ok())
                .and_then( |tok| Some(RequestCtx::new(ctx.event_loop, EventSet::readable(), tok)))
                .and_then( |req_ctx| Some((self.base.requests.get_mut(req_ctx.token), req_ctx)))
                .and_then( |(req, mut req_ctx)| Some(req.unwrap().ready(&mut req_ctx)));
        }
        if ctx.events.is_writable() {
            if self.base.responses.len() > 0 {
                self.send_all();
            }
        }
        // We are always listening for new requests. The server socket will be regregistered
        // as writable if there are responses to write
        self.base.reregister_server(ctx.event_loop, &self.server_socket, EventSet::readable());
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        self.base.request_ready(ctx);
        if self.base.responses.len() > 0 {
            self.send_all();
        }
    }

    fn send_all(&mut self) {
        debug!("There are {} responses to send", self.base.responses.len());
        self.base.responses.pop().and_then(|reply| Some(reply.send(&self.server_socket)));
    }
}
