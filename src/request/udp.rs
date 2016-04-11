extern crate bytes;
use mio::udp::UdpSocket;
use request::base::*;
use server_mio::RequestCtx;

pub struct UdpRequestFactory;
impl RequestFactory for UdpRequestFactory {
    
    fn new_with(&self, request: ForwardedRequestBase) -> Box<ForwardedRequest> {
        let req = UdpRequest {
            upstream_socket: None,
            base: request,
        };
        Box::new(req)
    }
}

//
// Encapsulates the components of a dns request and response over Udp.
//
pub struct UdpRequest {
    upstream_socket: Option<UdpSocket>,
    base: ForwardedRequestBase,
}

impl ForwardedRequest for UdpRequest {
  
    fn get(&self) -> &ForwardedRequestBase {
        &self.base
    }

    fn get_mut(&mut self) -> &mut ForwardedRequestBase {
        &mut self.base
    }

    fn accept(&mut self, ctx: &mut RequestCtx) -> Option<Response>{
        match UdpSocket::v4() {
            Ok(sock) => {
                let result = self.base.accept(ctx, &sock);
                self.upstream_socket = Some(sock);
                return result;
            },
            Err(e) => return Some(self.base.error_with(format!("Failed to create udp socket {:?}", e)))
        }
    }

    fn receive(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        debug_assert!(ctx.events.is_readable());
        let mut buf = [0; 4096];
        if let Some(ref sock) = self.upstream_socket {
            return match sock.recv_from(&mut buf) {
                Ok(Some((count, _))) => self.base.on_receive(ctx, count, &buf),
                Ok(None) => self.base.socket_debug(format!("No data received on upstream_socket. {:?}", ctx.token)),
                Err(e) => Some(self.base.on_receive_err(ctx, e))
            }
        }
        None
    }

    fn forward(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        if let Some(ref sock) = self.upstream_socket {
            return match sock.send_to(&mut self.base.query_buf.as_slice(), &self.base.params.upstream_addr) {
              Ok(Some(count)) => self.base.on_forward(ctx, count, sock),
              Ok(None) => self.base.socket_debug(format!("0 bytes sent. Staying in same state {:?}", ctx.token)),
              Err(e) => Some(self.base.on_forward_err(ctx, e))
            }
        }
        None
    }
}
