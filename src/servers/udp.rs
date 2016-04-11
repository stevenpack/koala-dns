use std::net::SocketAddr;
use std::collections::HashMap;
use mio::{Token, EventSet};
use mio::udp::UdpSocket;
use server_mio::{RequestCtx};
use request::base::*;
use request::udp::{UdpRequestFactory};
use servers::base::*;

pub struct UdpServer {
    pub server_socket: UdpSocket,
    pub base: ServerBase,
    accepted: HashMap<Token, SocketAddr>
}

impl UdpServer{
    pub const UDP_SERVER_TOKEN: Token = Token(1);
    pub fn new(addr: SocketAddr, max_connections: usize, params: RequestParams) -> UdpServer {
        let server_socket = Self::bind_udp(addr);
        let factory = Box::new(UdpRequestFactory);
        UdpServer {
            server_socket: server_socket,
            base: ServerBase::new(factory, params, Self::UDP_SERVER_TOKEN),
            accepted: HashMap::<Token, SocketAddr>::new()
        }
    }

    pub fn bind_udp(address: SocketAddr) -> UdpSocket {
        info!("Binding UDP to {:?}", address);
        let udp_socket = UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create udp socket {}", e));
        let _ = udp_socket.bind(&address).unwrap_or_else(|e| panic!("Failed to bind udp socket. Error was {}", e));
        udp_socket
    }

    pub fn accept(&mut self) -> Option<(SocketAddr,RawRequest)> {
        if let Some((addr, buf)) = self.receive(&self.server_socket) {
            let token = self.base.next_token();
            let req = RawRequest::new(token, buf);
            return Some((addr, req));
        }
        None
    }

    pub fn owns(&self, token: Token) -> bool {
        self.base.owns(token)
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
            if let Some((addr, mut req)) = self.accept() {                
                self.accepted.insert(req.token, addr);
                let mut req_ctx = RequestCtx::new(ctx.event_loop, EventSet::readable(), req.token, ctx.cache.clone());
                self.base.process(&mut req, &mut req_ctx);                    
            }
        }
        
        if self.base.responses.len() > 0 {
            self.send_all();
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
        while self.base.responses.len() > 0 {
            self.base.responses.pop().and_then(|reply| Some(self.send(&reply, &self.server_socket)));    
        }        
    }

    fn send(&self, response: &Response, socket: &UdpSocket) {
        if let Some(client_addr) = self.accepted.get(&response.token) {
            info!("{:?} bytes to send", response.bytes.len());
            match socket.send_to(&mut &response.bytes, &client_addr) {
                Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, &client_addr),
                Err(e) => error!("Failed to send. {:?} Error was {:?}", &client_addr, e),
            }            
        }
    }
}
