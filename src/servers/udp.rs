use std::net::SocketAddr;
use mio::Token;
use mio::util::Slab;
use mio::udp::UdpSocket;
use server_mio::RequestContext;
use request::request_base::{RequestBase, RequestParams};
use request::udp_request::UdpRequest;

pub struct UdpServer {
    pub server_socket: UdpSocket,
    pub requests: Slab<UdpRequest>,
    params: RequestParams
}
impl UdpServer {

    pub fn new(server_socket: UdpSocket, requests: Slab<UdpRequest>, params: RequestParams) -> UdpServer {
        UdpServer {
            server_socket: server_socket,
            requests: requests,
            params: params
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

    pub fn accept(&mut self, ctx: &RequestContext) -> Option<Token> {
        let new_tok = self.receive(&self.server_socket)
                          .and_then(|(addr, buf)| self.add_transaction(addr, buf.as_slice()));

        new_tok
        // if new_tok.is_some() {
        //     debug!("There are {:?} in-flight requests", self.requests.count());
        //     self.ready(ctx.event_loop, new_tok.unwrap(), ctx.events);
        // } else {
        //     error!("Failed to add request. New Token was None");
        // }
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

    fn add_transaction(&mut self, addr: SocketAddr, bytes: &[u8]) -> Option<Token> {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);

        // TODO: static
        let request = RequestBase::new(buf, self.params);

        // TODO: lose unwrap
        match self.requests.insert(UdpRequest::new(addr, request).unwrap()) {
            Ok(new_tok) => return Some(new_tok),
            Err(_) => {
                error!("Unable to start new transaction. Add to slab failed.");
                return None;
            }
        };
    }
}
