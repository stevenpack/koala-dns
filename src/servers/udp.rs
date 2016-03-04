use std::net::SocketAddr;
use mio::{EventLoop, Token, EventSet};
use mio::util::Slab;
use mio::udp::UdpSocket;
use server_mio::{MioServer,RequestContext};
use request::request_base::{RequestBase, RequestParams};
use request::udp_request::UdpRequest;

pub struct UdpServer {
    pub server_socket: UdpSocket,
    pub requests: Slab<UdpRequest>,
    responses: Vec<UdpRequest>,
    params: RequestParams
}

impl UdpServer {
    pub const UDP_SERVER_TOKEN: Token = Token(1);
    pub fn new(addr: SocketAddr, start_token: usize, max_connections: usize, params: RequestParams) -> UdpServer {
        let server_socket = Self::bind_udp(addr);
        let requests = Slab::new_starting_at(Token(start_token), max_connections);
        UdpServer {
            server_socket: server_socket,
            params: params,
            requests: requests,
            responses: Vec::<UdpRequest>::new()
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

    pub fn owns(&self, token: Token) -> bool {
        self.requests.contains(token)
    }

    pub fn accept(&mut self, ctx: &RequestContext) -> Option<UdpRequest> {
        return self.receive(&self.server_socket)
            .and_then(|(addr, buf)| Some(self.build_request(addr, buf.as_slice())));
    }

    fn receive(&self, socket: &UdpSocket) -> Option<(SocketAddr, Vec<u8>)> {
        // 2.3.4 Size Limits from RFC1035
        let mut buf = vec![0;512];
        match socket.recv_from(&mut buf) {
            Ok(Some((count, addr))) => {
                debug!("Received {} bytes from {}", count, addr);
                // trace!("{:?}", buf);
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

    pub fn server_ready(&mut self, ctx: &mut RequestContext)  {
        if ctx.events.is_readable() {
            self.accept(&ctx)
                .and_then( |req| self.requests.insert(req).ok())
                .and_then( |tok| Some(RequestContext::new(ctx.event_loop, EventSet::readable(), tok)))
                .and_then( |req_ctx| Some((self.requests.get_mut(req_ctx.token), req_ctx)))
                .and_then( |(req, mut req_ctx)| Some(req.unwrap().ready(&mut req_ctx)));
        }
        if ctx.events.is_writable() {
            self.send_reply();
        }
        // We are always listening for new requests. The server socket will be regregistered
        // as writable if there are responses to write
        self.reregister_server(ctx.event_loop, EventSet::readable());
    }

    pub fn request_ready(&mut self, ctx: &mut RequestContext) {
        let mut queue_response = false;
        match self.requests.get_mut(ctx.token) {
            Some(mut request) => {
                request.ready(ctx);
                queue_response = request.inner.has_reply();
            }
            None => {/* must be a tcp request*/},
        }
        if queue_response {
            self.queue_response(ctx.token);
            self.reregister_server(ctx.event_loop, EventSet::readable() | EventSet::writable());
        }
    }

    fn queue_response(&mut self, token: Token) {
        self.requests.remove(token).and_then(|req| Some(self.responses.push(req)));
    }

    fn send_reply(&mut self) {
        debug!("There are {} responses to send", self.responses.len());
        self.responses.pop().and_then(|reply| Some(reply.send(&self.server_socket)));
    }

    fn build_request(&mut self, addr: SocketAddr, bytes: &[u8]) -> UdpRequest {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = RequestBase::new(buf, self.params);

        UdpRequest::new(addr, request)
    }

    fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, events: EventSet) {
        MioServer::reregister_server(event_loop, events, UdpServer::UDP_SERVER_TOKEN, &self.server_socket);
    }
}
