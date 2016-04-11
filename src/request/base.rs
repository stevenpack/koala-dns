use std::net::SocketAddr;
use std::io::Error;
use mio::{Token, Timeout, Handler, EventSet, Evented, PollOpt};
use server_mio::RequestCtx;
use dns::dns_entities::*;


//Request
 // Token, State, QueryBuf, Query
//ForwardedRequest
 // Token, QueryBuf,Params,TimeoutHandle
//Response
// Token, Response, ResponseBuf
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

pub trait RequestFactory {
    fn new_with(&self, request: ForwardedRequestBase) -> Box<ForwardedRequest>;
}

pub trait ForwardedRequest {
    //fn new_with(request: RequestBase) -> T;
    fn get(&self) -> &ForwardedRequestBase;
    fn get_mut(&mut self) -> &mut ForwardedRequestBase;

    fn ready(&mut self, ctx: &mut RequestCtx) -> Option<Response> {
        debug!("State {:?} {:?} {:?}", self.get().state, ctx.token, ctx.events);
        // todo: authorative? cached? forward?
        let opt_response = None;
        match self.get().state {
            ForwardedRequestState::New => self.accept(ctx),
            ForwardedRequestState::Accepted => self.forward(ctx),
            ForwardedRequestState::Forwarded => opt_response = self.receive(ctx),
            _ => debug!("Nothing to do for this state {:?}", self.get().state),
        }
        opt_response
    }
   
    fn accept(&mut self, ctx: &mut RequestCtx);
    fn forward(&mut self, ctx: &mut RequestCtx);
    fn receive(&mut self, ctx: &mut RequestCtx) -> Option<Response>;
}

pub struct Response {
    pub token: Token,
    pub bytes: Vec<u8>, //answer without the length prefix
    pub msg: DnsMessage,
}

impl Response {
    pub fn new(token: Token, bytes: Vec<u8>, msg: DnsMessage) -> Response {
        Response {
            token: token,
            bytes: bytes,
            msg: msg
        }
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ForwardedRequestState {
    New,
    Accepted,
    Parsed,
    Forwarded,
    ResponseReceived,
    Error,
}

//RequestMixin
pub struct ForwardedRequestBase {
    pub token: Token,
    pub state: ForwardedRequestState,
    pub query_buf: Vec<u8>, //query without the length prefix
    pub query: Option<DnsMessage>,
    // pub response_buf: Option<Vec<u8>>, //answer without the length prefix
    // pub response: Option<DnsMessage>,
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
        return ForwardedRequestBase {
            token: token,
            state: ForwardedRequestState::New,
            query: None,
            query_buf: query_buf,
            // response_buf: None,
            // response: None,
            timeout_handle: None,
            params: params,
        };
    }

    pub fn set_state(&mut self, state: ForwardedRequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    pub fn set_timeout_handle(&mut self, timeout: Timeout) {
        self.timeout_handle = Some(timeout);
    }

    pub fn on_timeout(&mut self, token: Token) -> Response {
        self.error_with(format!("{:?} timed out", token));
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
        //TODO: parse here, or on adding to cache? As that's what it's for...
        Response::new(self.token, bytes, DnsMessage::parse(&bytes))
    }

    pub fn error_with(&mut self, err_msg: String) -> Response {
        self.set_state(ForwardedRequestState::Error);
        debug!("{}", err_msg);
        let req = DnsMessage::parse(&self.query_buf);
        let reply = DnsHeader::new_error(req.header, 2);
        let bytes = reply.to_bytes();
        Response::new(self.token, bytes, reply)
    }

    pub fn has_reply(&self) -> bool {
        return self.response_buf.is_some() || self.response.is_some();
    }

    pub fn accept(&mut self, ctx: &mut RequestCtx, sock: &Evented) {
        debug_assert!(ctx.events.is_readable());
        self.set_state(ForwardedRequestState::Accepted);
        //debug!("{:?}", DnsMessage::parse(&self.query_buf));
        //todo: if need to forward...
        self.register_upstream(ctx, EventSet::writable(), sock);
        debug!("Accepted and registered upstream");
    }

    pub fn on_receive(&mut self, ctx: &mut RequestCtx, count: usize, buf: &[u8]) -> Option<Response> {
       if count > 0 {
           //trace!("{:#?}", DnsMessage::parse(buf));           
           self.clear_timeout(ctx);
           self.set_state(ForwardedRequestState::ResponseReceived);
           return self.buffer_response(&buf, count);
       } 
       warn!("No data received on upstream_socket. {:?}", ctx.token);
       None
    }
    pub fn on_receive_err(&mut self, ctx: &mut RequestCtx, e: Error) {
        self.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e));
        self.clear_timeout(ctx);
    }

    pub fn on_forward(&mut self, ctx: &mut RequestCtx, count: usize, sock: &Evented) {
        debug!("Sent {:?} bytes", count);
        self.set_state(ForwardedRequestState::Forwarded);
        self.register_upstream(ctx, EventSet::readable(), sock);
        // TODO: No, don't just timeout forwarded requests, time out the whole request,
        // be it cached, authorative or forwarded
        self.set_timeout(ctx);
    }

    pub fn on_forward_err(&mut self, ctx: &mut RequestCtx, e: Error) {
        self.error_with(format!("Failed to write to upstream_socket. {:?} {:?}", e, ctx.token))
    }
}
