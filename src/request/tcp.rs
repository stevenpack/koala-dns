use mio::EventSet;
use mio::tcp::{TcpStream};
use std::net::SocketAddr;
use std::io::{Read, Write};
use request::base::{RequestState, RequestBase};
//use std::collections::HashMap;
//use dns::dns_entities::DnsMessage;
use server_mio::RequestContext;
use request::base::IRequest;

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    pub client_addr: SocketAddr,
    pub inner: RequestBase,
}

impl IRequest<TcpRequest> for TcpRequest {
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
}

impl TcpRequest {


    fn accept(&mut self, ctx: &mut RequestContext) {
        self.upstream_socket = TcpStream::connect(&self.inner.params.upstream_addr).ok();
        debug!("upstream created");
        match self.upstream_socket {
            Some(ref sock) => self.inner.accept(ctx, sock),
            None => {}
        }
    }

    pub fn ready(&mut self, ctx: &mut RequestContext) {
        debug!("State {:?} {:?} {:?}",
               self.inner.state,
               ctx.token,
               ctx.events);
        // todo: authorative? cached? forward?
        match self.inner.state {
            RequestState::New => self.accept(ctx),
            RequestState::Accepted => self.forward(ctx),
            RequestState::Forwarded => self.receive(ctx),
            _ => debug!("Nothing to do for this state {:?}", self.inner.state),
        }
    }
    fn forward(&mut self, ctx: &mut RequestContext) {
        debug!("Forwarding... {:?} bytes to forward", self.inner.query_buf.len());
        debug_assert!(ctx.events.is_writable());
        //TODO: error on fail to create upstream socket
        match self.upstream_socket {
            Some(ref mut sock) => {
                match sock.write(&mut self.inner.query_buf.as_slice()) {
                      Ok(count) => {
                          if count > 0 {
                              //TODO base.on_forwarded
                              self.inner.set_state(RequestState::Forwarded);
                              self.inner.register_upstream(ctx, EventSet::readable(), sock);
                              // TODO: No, don't just timeout forwarded requests, time out the whole request,
                              // be it cached, authorative or forwarded
                              self.inner.set_timeout(ctx);
                          } else {
                              warn!("0 bytes sent. Staying in same state {:?}", ctx.token);
                          }
                      }
                      Err(e) => {
                          //todo: base.on_forward_error
                          self.inner.error_with(format!("Failed to write to upstream_socket. {:?} {:?}",
                                                        e,
                                                        ctx.token))
                      }
                  }
            },
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
