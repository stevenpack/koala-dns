use std::net::SocketAddr;
use std::io::Error;
use mio::{Token, Timeout, Handler, EventSet, Evented, PollOpt};
use server_mio::RequestCtx;
use dns::dns_entities::*;
use std::sync::{Arc, RwLock};
use cache::*;
use buf::*;

pub trait Request<T> {
    fn new_with(client_addr: SocketAddr, request: RequestBase) -> T;
    fn get(&self) -> &RequestBase;
    fn get_mut(&mut self) -> &mut RequestBase;

    fn ready(&mut self, ctx: &mut RequestCtx) {
        debug!("State {:?} {:?} {:?}", self.get().state, ctx.token, ctx.events);
        // todo: authorative? cached? forward?
        match self.get().state {
            RequestState::New => self.accept(ctx),
            RequestState::Accepted => self.forward(ctx),
            RequestState::Forwarded => self.receive(ctx),
            _ => debug!("Nothing to do for this state {:?}", self.get().state),
        }
    }

    fn ready_cache(&mut self, ctx: &mut RequestCtx, cache_lock: Arc<RwLock<Cache>>) {
        //if not in cache...
        match cache_lock.read() {
            Ok(cache) => {
                let query = DnsMessage::parse(&self.get().query_buf);
                let key = CacheKey::from(&query.question);
                if let Some(entry) = cache.get(&key) {
                    
                    //TODO: need to adjust the TTL down?
                    //TODO: cache the whole message?
                    let mut answer_header = query.header.clone();
                    answer_header.id = query.header.id;
                    answer_header.qr = true;
                    answer_header.ancount = entry.answers.len() as u16;
                    let msg = DnsMessage::new_reply(answer_header, query.question, entry.answers.clone());
                    debug!("Could answer with {:?} based on key {:?}", msg, entry.key);
                    self.get_mut().response_buf = Some(msg.to_bytes());
                    self.get_mut().response = Some(msg);                    
                    self.get_mut().state = RequestState::ResponseFromCache;
                } 
                self.ready(ctx);
            }
            Err(e) => error!("Couldn't get read lock {:?}", e)
        }
    }

    fn accept(&mut self, ctx: &mut RequestCtx);
    fn forward(&mut self, ctx: &mut RequestCtx);
    fn receive(&mut self, ctx: &mut RequestCtx);
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived,
    ResponseFromCache,
    Error,
}

//RequestMixin
pub struct RequestBase {
    pub token: Token,
    pub state: RequestState,
    pub query_buf: Vec<u8>, //query without the length prefix
    //pub query: Option<DnsMessage>,
    pub response_buf: Option<Vec<u8>>, //answer without the length prefix
    pub response: Option<DnsMessage>,
    pub timeout_handle: Option<Timeout>,
    pub params: RequestParams,
}

#[derive(Copy)]
#[derive(Clone)]
pub struct RequestParams {
    pub timeout: u64,
    pub upstream_addr: SocketAddr,
}

impl RequestBase {
    pub fn new(token: Token, query_buf: Vec<u8>, params: RequestParams) -> RequestBase {
        return RequestBase {
            token: token,
            state: RequestState::New,
            //query: None,
            query_buf: query_buf,
            response_buf: None,
            response: None,
            timeout_handle: None,
            params: params,
        };
    }

    pub fn set_state(&mut self, state: RequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    pub fn set_timeout_handle(&mut self, timeout: Timeout) {
        self.timeout_handle = Some(timeout);
    }

    pub fn on_timeout(&mut self, token: Token) {
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

    pub fn buffer_response(&mut self, buf: &[u8], count: usize) {
        let mut response = Vec::with_capacity(count);
        response.extend_from_slice(&buf);
        response.truncate(count);
        self.response_buf = Some(response);
        debug!("buffered {:?} bytes for response", count);
    }

    pub fn error_with(&mut self, err_msg: String) {
        self.set_state(RequestState::Error);
        debug!("{}", err_msg);
        let req = DnsMessage::parse(&self.query_buf);
        let reply = DnsHeader::new_error(req.header, 2);
        self.response_buf = Some(reply.to_bytes());
    }

    pub fn has_reply(&self) -> bool {
        return self.response_buf.is_some() || self.response.is_some();
    }

    pub fn accept(&mut self, ctx: &mut RequestCtx, sock: &Evented) {
        debug_assert!(ctx.events.is_readable());
        self.set_state(RequestState::Accepted);
        debug!("{:?}", DnsMessage::parse(&self.query_buf));
        //todo: if need to forward...
        self.register_upstream(ctx, EventSet::writable(), sock);
        debug!("Accepted and registered upstream");
    }

    pub fn on_receive(&mut self, ctx: &mut RequestCtx, count: usize, buf: &[u8]) {
       if count > 0 {
           //trace!("{:#?}", DnsMessage::parse(buf));
           self.buffer_response(&buf, count);
           self.clear_timeout(ctx);
           self.set_state(RequestState::ResponseReceived);
       } else {
           warn!("No data received on upstream_socket. {:?}", ctx.token);
       }
    }
    pub fn on_receive_err(&mut self, ctx: &mut RequestCtx, e: Error) {
        self.error_with(format!("Receive failed on {:?}. {:?}", ctx.token, e));
        self.clear_timeout(ctx);
    }

    pub fn on_forward(&mut self, ctx: &mut RequestCtx, count: usize, sock: &Evented) {
        debug!("Sent {:?} bytes", count);
        self.set_state(RequestState::Forwarded);
        self.register_upstream(ctx, EventSet::readable(), sock);
        // TODO: No, don't just timeout forwarded requests, time out the whole request,
        // be it cached, authorative or forwarded
        self.set_timeout(ctx);
    }

    pub fn on_forward_err(&mut self, ctx: &mut RequestCtx, e: Error) {
        self.error_with(format!("Failed to write to upstream_socket. {:?} {:?}", e, ctx.token))
    }
}
