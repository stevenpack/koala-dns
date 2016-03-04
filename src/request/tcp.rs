use mio::EventSet;
use mio::tcp::{TcpStream, TcpListener};
use std::net::SocketAddr;
use request::base::{RequestState, RequestBase, RequestParams};
use std::collections::HashMap;
//use dns::dns_entities::DnsMessage;
use server_mio::RequestContext;

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    pub client_addr: SocketAddr,
    pub inner: RequestBase,
}

impl TcpRequest {
    pub fn new(client_addr: SocketAddr, request: RequestBase) -> TcpRequest {
        return TcpRequest {
            upstream_socket: None,
            client_addr: client_addr,
            inner: request,
        };
    }

    fn accept(&mut self, ctx: &mut RequestContext) {
        // debug_assert!(ctx.events.is_readable());
        // self.inner.set_state(RequestState::Accepted);
        //
        // match self.tcp_server.accept() {
        //     Ok(Some((stream, addr))) => {
        //         match self.add_transaction(addr, Vec::<u8>::new().as_slice(), TCP_SERVER_TOKEN) {
        //             Some(tok) => {
        //                 Self::register(event_loop, &stream, EventSet::readable(), tok, true);
        //                 self.pending.insert(tok, stream);
        //             }
        //             None => error!("add_transaction failed"),
        //         }
        //     }
        //     Ok(None) => debug!("Socket would block waiting..."),
        //     Err(err) => error!("Failed to accept tcp connection {}", err),
        // }
    }
}
