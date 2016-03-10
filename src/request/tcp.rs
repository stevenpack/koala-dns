use mio::EventSet;
use mio::tcp::{TcpStream};
use std::net::SocketAddr;
use std::io::{Read, Write};
use request::base::{RequestState, RequestBase};
//use std::collections::HashMap;
//use dns::dns_entities::DnsMessage;
use server_mio::RequestContext;
use request::base::Request;

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    client_addr: SocketAddr,
    inner: RequestBase,
}

impl Request<TcpRequest> for TcpRequest {
    fn new_with(client_addr: SocketAddr, request: RequestBase) -> TcpRequest {
        return TcpRequest {
            upstream_socket: None,
            client_addr: client_addr,
            inner: request,
        };
    }

    fn get(&self) -> &RequestBase {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut RequestBase {
        &mut self.inner
    }

    fn accept(&mut self, ctx: &mut RequestContext) {
        self.upstream_socket = TcpStream::connect(&self.inner.params.upstream_addr).ok();
        debug!("upstream created");
        match self.upstream_socket {
            Some(ref sock) => self.inner.accept(ctx, sock),
            None => {}
        }
    }

    fn receive(&mut self, ctx: &mut RequestContext) {
        assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];

        match self.upstream_socket {
            Some(ref mut socket) => {
                match socket.read(&mut buf) {
                    Ok(count) => {
                        if count > 0 {
                            //TODO: base.on_received()
                            debug!("Received {} bytes", count);
                            //trace!("{:#?}", DnsMessage::parse(&buf));
                            self.inner.buffer_response(&buf, count);
                            self.inner.clear_timeout(ctx);
                            self.inner.set_state(RequestState::ResponseReceived);
                        } else {
                            warn!("No data received on upstream_socket. {:?}", ctx.token);
                        }
                    }
                    Err(e) => {
                        //TODO: base.on_received_error()
                        self.inner.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e));
                        self.inner.clear_timeout(ctx);
                    }
                }
            }
            None => error!("tcp stream"),
        }
    }

    fn forward(&mut self, ctx: &mut RequestContext) {
        debug!("Forwarding... {:?} bytes to forward", self.inner.query_buf.len());
        debug_assert!(ctx.events.is_writable());
        //TODO: error on fail to create upstream socket
        match self.upstream_socket {
            Some(ref mut sock) => {
                // prefix with length
            let len = self.inner.query_buf.len() as u8;
            self.inner.query_buf.insert(0, len);
            self.inner.query_buf.insert(0, 0);

            info!("{:?} bytes to send", self.inner.query_buf.len());
                match sock.write_all(&mut self.inner.query_buf.as_slice()) {
                      Ok(count) => {
                          debug!("Sent {:?} bytes", count);
                          //TODO base.on_forwarded
                          self.inner.set_state(RequestState::Forwarded);
                          self.inner.register_upstream(ctx, EventSet::readable(), sock);
                          // TODO: No, don't just timeout forwarded requests, time out the whole request,
                          // be it cached, authorative or forwarded
                          self.inner.set_timeout(ctx);

                      }
                      Err(e) => {
                          //todo: base.on_forward_error
                          self.inner.error_with(format!("Failed to write to upstream_socket. {:?} {:?}",
                                                        e,
                                                        ctx.token))
                      }
                  }
            },
            None => error!("tcp upstream socket")
        }
    }
}

impl TcpRequest {

    pub fn send(&self, socket: &mut TcpStream) {
        match self.inner.response_buf {
            Some(ref response) => {
                info!("{:?} bytes to send", response.len());

                match socket.write(&mut &response.as_slice()) {
                    Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
                    Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
                }
            }
            None => error!("Trying to send before a response has been buffered."),
        }
    }

}
