
use mio::tcp::{TcpStream};
use std::io::{Read, Write};
use request::base::*;
use server_mio::RequestCtx;

pub struct TcpRequestFactory;
impl RequestFactory for TcpRequestFactory {
    
    fn new_with(&self, request: ForwardedRequestBase) -> Box<ForwardedRequest> {
        let req = TcpRequest {
            upstream_socket: None,
            base: request,
        };
        Box::new(req)
    }
}

pub struct TcpRequest {
    upstream_socket: Option<TcpStream>,
    base: ForwardedRequestBase,
}

impl ForwardedRequest for TcpRequest {

    fn get(&self) -> &ForwardedRequestBase {
        &self.base
    }

    fn get_mut(&mut self) -> &mut ForwardedRequestBase {
        &mut self.base
    }

    fn accept(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        let addr = self.base.params.upstream_addr;
        return match TcpStream::connect(&addr) {
            Ok(sock) => {
                self.base.accept(ctx, &sock);
                self.upstream_socket = Some(sock);
                return None;
            },
            Err(e) => Some(self.base.error_with(format!("Failed to connect to {:?} {:?}", addr, e)))
        }
    }

    fn receive(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        debug_assert!(ctx.events.is_readable());
        const PREFIX_LEN: usize = 2;
        let mut buf = [0; 4096];
        if let Some(ref mut sock) = self.upstream_socket {
            return match sock.read(&mut buf) {
                Ok(count) => {
                    //store the response without the prefix
                    debug!("Received {} bytes", count);
                    if count < PREFIX_LEN {
                        warn!("tcp: Only received length prefix. No content");
                        return None;
                    }
                    self.base.on_receive(ctx, count - PREFIX_LEN , &buf[PREFIX_LEN..count])
                 },
                Err(e) => Some(self.base.on_receive_err(ctx, e))
            }
        }
        None
    }

    fn forward(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        debug_assert!(ctx.events.is_writable());
        if let Some(ref mut sock) = self.upstream_socket {
            // prefix with length
            Self::prefix_with_length(&mut self.base.query_buf);
            let len = self.base.query_buf.len() as usize;
            debug!("{:?} bytes to send (inc 2b prefix)", len);
            return match sock.write_all(&mut self.base.query_buf.as_slice()) {
                Ok(_) => self.base.on_forward(ctx, len, sock),
                Err(e) => Some(self.base.on_forward_err(ctx, e))
            }
        }
        None
    }
}

impl TcpRequest {
    
    //TODO: duplicated
    fn prefix_with_length(buf: &mut Vec<u8>) {
        //TCP responses are prefixed with a 2-byte length
        let len = buf.len() as u16;
        buf.insert(0, len as u8);
        buf.insert(0, len.swap_bytes() as u8);
        debug!("Added 2b prefix of len: {:?}", len);
    }
}
