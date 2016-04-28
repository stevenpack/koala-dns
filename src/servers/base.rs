use std::collections::HashMap;
use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use server_mio::{MioServer,RequestCtx};
use request::base::*;
use cache::*;
use dns::message::*;
use servers::pipeline::*;


pub struct ServerBase {
    pub request_factory: Box<RequestFactory>,
    pub forwarded: HashMap<Token, Box<ForwardedRequest>>,
    pub responses: Vec<Response>,
    pub params: RequestParams,
    server_token: Token,
    last_request: Token,
    pipeline: RequestPipeline,
    max_connections: usize,
}

impl ServerBase {

    const REQUEST_TOKEN_START: usize = 10;

    pub fn new(factory: Box<RequestFactory>, params: RequestParams, token: Token, max_connections: usize) -> ServerBase {
        debug!("New server listening on {:?}", token);
        ServerBase {            
            request_factory: factory,
            forwarded: HashMap::<Token, Box<ForwardedRequest>>::new(), //TODO: max forwards setting
            responses: Vec::<Response>::new(),
            params: params,
            server_token: token,
            last_request: Token(Self::REQUEST_TOKEN_START), //Some number clearly different from the starting token
            pipeline: RequestPipeline::default(),
            max_connections: max_connections
        }
    }

    pub fn process(&mut self, mut request: &mut RawRequest, mut ctx: &mut RequestCtx) {
        if let Some(response) = self.pipeline.process(&mut request, ctx) {
            self.queue_response(ctx, response);
            return;            
        } 
        //No response, forward upstream
        let mut forward = self.build_forward_request(request.token, &request.bytes);
        debug!("Added {:?} to forwarded", forward.get().token);
        if let Some(response) = forward.ready(&mut ctx) {
            //Could get an error straight off...
            self.queue_response(&ctx, response);
            return;
        }
        self.forwarded.insert(forward.get().token, forward);
    }

    pub fn register(&self, event_loop: &mut EventLoop<MioServer>,
                socket: &Evented,
                events: EventSet,
                token: Token,
                reregister: bool) {

        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        if reregister {
            let reg = event_loop.reregister(socket, token, events, poll_opt);
            debug!("Re-registered {:?} {:?} {:?}", token, events, reg);
        } else {
            let reg = event_loop.register(socket, token, events, poll_opt);
            debug!("Registered {:?} {:?} {:?}", token, events, reg);
        }
    }

    pub fn reregister_server(&self, event_loop: &mut EventLoop<MioServer>, sock: &Evented, events: EventSet) {
        self.register(event_loop, sock, events, self.server_token, true);
    }

    pub fn queue_response(&mut self, ctx: &RequestCtx, response: Response) {
        if response.source == Source::Upstream {
            debug!("Upstream response. Will cache...");
            if let Some(cache_entry) = CacheEntry::from(&response.msg) {
                ctx.cache.write().unwrap().upsert(cache_entry.key.clone(), cache_entry);    
            }            
        }
        self.responses.push(response);
        debug!("queued response {:?}", ctx.token);        
    }

    pub fn build_forward_request(&mut self, token: Token, bytes: &[u8]) -> Box<ForwardedRequest> {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = ForwardedRequestBase::new(token, buf, self.params);
        self.request_factory.new_with(request)
    }

    pub fn timeout(&mut self, ctx: &mut RequestCtx) {
        debug!("Timeout for {:?} {:?}", ctx.token, ctx.events);
        let mut opt_response = None;
        if let Some(mut req) = self.forwarded.get_mut(&ctx.token) {
            opt_response = Some(req.get_mut().on_timeout(ctx.token));            
        } else {
            warn!("Timeout for {:?}, but not in forwarded map", ctx.token);
        }
        if let Some(response) = opt_response {
            self.queue_response(ctx, response);
        }
    }

    pub fn owns(&self, token: Token) -> bool {
        self.forwarded.contains_key(&token)
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        debug!("Request for {:?} {:?}", ctx.token, ctx.events);
        let mut response_opt = None;
        if let Some(ref mut request) = self.forwarded.get_mut(&ctx.token) {
            response_opt = request.ready(ctx);            
        }
        if let Some(response) = response_opt {
            self.forwarded.remove(&ctx.token);
            self.queue_response(&ctx, response);            
        }
    }

    pub fn next_token(&mut self) -> Token {
        if self.last_request.as_usize() > self.max_connections {
            //TODO: Naive... need to enforce max connections at forward time
            self.last_request = Token(Self::REQUEST_TOKEN_START);
        }
        self.last_request = Token(self.last_request.as_usize() + 1);
        debug!("next_token gave -> {:?}", self.last_request);
        self.last_request
    }
}
