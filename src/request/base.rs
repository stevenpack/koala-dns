use std::net::SocketAddr;
use std::io::Error;
use mio::{Token, Timeout, Handler, EventSet, Evented, PollOpt};
use server_mio::RequestCtx;
use dns::dns_entities::*;

pub struct RawRequest {
    pub token: Token,
    pub bytes: Vec<u8>,
    pub query: Option<DnsMessage>
}

impl RawRequest {
    pub fn new(token: Token, bytes: Vec<u8>) -> RawRequest {
        RawRequest {
            token: token,
            bytes: bytes,
            query: None
        }
    }
}

#[derive(PartialEq)]
pub enum Source {
    System, //such as error
    Authoritive,
    Cache,
    Upstream
}

pub struct Response {
    pub token: Token,
    pub bytes: Vec<u8>, //answer without the length prefix
    pub msg: DnsMessage,
    pub source: Source
}

impl Response {
    // pub fn new(token: Token, bytes: Vec<u8>, msg: DnsMessage) -> Response {
    //     Self::with_source(token, bytes, msg, Source::System)
    // }
     pub fn with_source(token: Token, bytes: Vec<u8>, msg: DnsMessage, source: Source) -> Response {
        Response {
            token: token,
            bytes: bytes,
            msg: msg,
            source: source
        }
    }
}

pub trait RequestFactory {
    fn new_with(&self, request: ForwardedRequestBase) -> Box<ForwardedRequest>;
}

pub trait ForwardedRequest {
    //fn new_with(request: RequestBase) -> T;
    fn get(&self) -> &ForwardedRequestBase;
    fn get_mut(&mut self) -> &mut ForwardedRequestBase;

    fn ready(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        debug!("ForwardRequest State {:?} {:?} {:?}", self.get().state, ctx.token, ctx.events);
        match self.get().state {
            ForwardedRequestState::New => self.accept(ctx),
            ForwardedRequestState::Accepted => self.forward(ctx),
            ForwardedRequestState::Forwarded => self.receive(ctx),
            _ => {
                debug!("Nothing to do for this state {:?}", self.get().state);
                None
            },
        }
    }
   
    fn accept(&mut self, ctx: &mut RequestCtx) -> Option<Response>;
    fn forward(&mut self, ctx: &mut RequestCtx) -> Option<Response>;
    fn receive(&mut self, ctx: &mut RequestCtx) -> Option<Response>;
}



#[derive(Debug)]
#[derive(PartialEq)]
pub enum ForwardedRequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived,
    Error,
}

pub struct ForwardedRequestBase {
    pub token: Token,
    pub state: ForwardedRequestState,
    pub query_buf: Vec<u8>, //query without the length prefix
    pub query: Option<DnsMessage>,
    pub timeout_handle: Option<Timeout>,
    pub params: RequestParams,
}

#[derive(Copy)]
#[derive(Clone)]
pub struct RequestParams {
    pub timeout: u64,
    pub upstream_addr: SocketAddr,
}

impl ForwardedRequestBase {
    pub fn new(token: Token, query_buf: Vec<u8>, params: RequestParams) -> ForwardedRequestBase {
        ForwardedRequestBase {
            token: token,
            state: ForwardedRequestState::New,
            query: None,
            query_buf: query_buf,
            timeout_handle: None,
            params: params,
        }
    }

    pub fn set_state(&mut self, state: ForwardedRequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    pub fn set_timeout_handle(&mut self, timeout: Timeout) {
        self.timeout_handle = Some(timeout);
    }

    pub fn on_timeout(&mut self, token: Token) -> Response {
        self.error_with(format!("{:?} timed out", token))
    }

    pub fn set_timeout(&mut self, ctx: &mut RequestCtx) {
        match ctx.event_loop.timeout_ms(ctx.token, self.params.timeout) {
            Ok(t) => self.set_timeout_handle(t),
            Err(e) => error!("Failed to schedule timeout for {:?}. {:?}", ctx.token, e),
        }
    }

    pub fn clear_timeout(&mut self, ctx: &mut RequestCtx) {
        if let Some(handle) = self.timeout_handle {
            let result = ctx.event_loop.clear_timeout(handle);
            debug!("Timeout cleared result for {:?}={:?}", ctx.token, result);
        }
    }

    pub fn register_upstream(&mut self, ctx: &mut RequestCtx, events: EventSet, sock: &Evented) {
        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        match ctx.event_loop.register(sock, ctx.token, events, poll_opt) {
            Ok(_) => debug!("Registered upstream {:?} {:?}", ctx.token, events),
            Err(e) => error!("Failed to register upstream socket. {}", e)
        }        
    }

    pub fn buffer_response(&mut self, buf: &[u8], count: usize) -> Response  {
        let mut bytes = Vec::with_capacity(count);
        bytes.extend_from_slice(&buf);
        bytes.truncate(count);        
        debug!("buffered {:?} bytes for response", count);
        let msg = DnsMessage::parse(&bytes);
        Response::with_source(self.token, bytes, msg, Source::Upstream)
    }

    pub fn error_with(&mut self, err_msg: String) -> Response {
        self.set_state(ForwardedRequestState::Error);
        debug!("{}", err_msg);
        let req = DnsMessage::parse(&self.query_buf);
        let header = DnsHeader::new_error(req.header, 4);
        let msg = DnsMessage::new_error(header);
        let bytes = msg.to_bytes();
        Response::with_source(self.token, bytes, msg, Source::System)
    }

    pub fn accept(&mut self, ctx: &mut RequestCtx, sock: &Evented) -> Option<Response> {
        debug_assert!(ctx.events.is_readable());
        self.set_state(ForwardedRequestState::Accepted);
        self.register_upstream(ctx, EventSet::writable(), sock);
        debug!("Accepted and registered upstream");
        None
    }

    pub fn on_receive(&mut self, ctx: &mut RequestCtx, count: usize, buf: &[u8]) -> Option<Response> {
       if count > 0 {
           self.clear_timeout(ctx);
           self.set_state(ForwardedRequestState::ResponseReceived);
           return Some(self.buffer_response(&buf, count));
       } 
       warn!("No data received on upstream_socket. {:?}", ctx.token);
       None
    }
    
    pub fn on_receive_err(&mut self, ctx: &mut RequestCtx, e: Error) -> Response {
        self.clear_timeout(ctx);
        self.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e))        
    }

    pub fn socket_debug(&self, debug_str: String) -> Option<Response> {
        debug!("{}", debug_str);
        None
    }

    pub fn on_forward(&mut self, ctx: &mut RequestCtx, count: usize, sock: &Evented) -> Option<Response> {
        debug!("Sent {:?} bytes", count);
        self.set_state(ForwardedRequestState::Forwarded);
        self.register_upstream(ctx, EventSet::readable(), sock);
        // TODO: No, don't just timeout forwarded requests, time out the whole request,
        // be it cached, authorative or forwarded
        self.set_timeout(ctx);
        None
    }

    pub fn on_forward_err(&mut self, ctx: &mut RequestCtx, e: Error) -> Response {
        self.error_with(format!("Failed to write to upstream_socket. {:?} {:?}", e, ctx.token))
    }
}
