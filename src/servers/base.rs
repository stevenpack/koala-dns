use std::collections::HashMap;
use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use server_mio::{MioServer,RequestCtx};
use request::base::*;
use cache::*;
//use std::net::SocketAddr;
use dns::dns_entities::*;

pub trait PipelineStage {
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response>;
}

// pub struct PipelineResult {
//     response: Option<Response>,
//     forwarded: Option<Box<ForwardedRequest>>
// }

pub struct RequestPipeline {
    stages: Vec<Box<PipelineStage>>
}

struct ParseStage;
struct AuthorityStage;
struct CacheStage;
struct ForwardStage;

impl RequestPipeline {
    fn new() -> RequestPipeline {
        
        let mut stages = Vec::<Box<PipelineStage>>::new();
        stages.push(Box::new(ParseStage));
        stages.push(Box::new(AuthorityStage));
        stages.push(Box::new(CacheStage));
        stages.push(Box::new(ForwardStage));
        
        RequestPipeline {
            stages: stages
        }
    }
}

impl PipelineStage for RequestPipeline {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {
        for stage in self.stages.iter() {
            if let Some(response) = stage.process(request, ctx) {
                return Some(response)
            }
        }
        None
    }
}

impl PipelineStage for ParseStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        //TODO: DnsMessage::parse should be Result. If it fails, we shoudl return a fail response here
        request.query = Some(DnsMessage::parse(&request.bytes));
        debug!("Parsed query");
        None
    }
}

impl PipelineStage for AuthorityStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("No Master File parsing yet, so no authoritative records");
        None
    }
}

impl PipelineStage for CacheStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Entered cahce stage");
        match ctx.cache.read() {
            Ok(cache) => {
                let query = DnsMessage::parse(&request.bytes);
                if let Some(question) = query.first_question() {
                    let key = CacheKey::from(&question);
                    if let Some(entry) = cache.get(&key) {
                        //request.state = ForwardedRequestState::ResponseFromCache;
                        //TODO: need to adjust the TTL down?
                        //TODO: cache the whole message?
                        let mut answer_header = query.header.clone();
                        answer_header.id = query.header.id;
                        answer_header.qr = true;
                        answer_header.ancount = entry.answers.len() as u16;
                        let msg = DnsMessage::new_reply(answer_header, query.questions.clone(), entry.answers.clone());
                        debug!("Could answer with {:?} based on key {:?}", msg, entry.key);
                        let bytes = msg.to_bytes();
                        return Some(Response::new(ctx.token, bytes, msg));
                    } 
                }
            }
            Err(e) => error!("Couldn't get read lock {:?}", e)
        }
        debug!("No cache hit");
        None
    }
}

impl PipelineStage for ForwardStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Forward does nothing. Create ForwardRequest from RequestRaw...");
        None 
    }
}

pub struct ServerBase {
    pub request_factory: Box<RequestFactory>,
    pub forwarded: HashMap<Token, Box<ForwardedRequest>>,
    pub responses: Vec<Response>,
    pub params: RequestParams,
    //socket: Box<Evented> //could add socket with box so server registration happens here
    server_token: Token,
    last_request: Token,
    pipeline: RequestPipeline
}

impl ServerBase {
    pub fn new(factory: Box<RequestFactory>, params: RequestParams, token: Token) -> ServerBase {
        debug!("New server listening on {:?}", token);
        ServerBase {            
            request_factory: factory,
            forwarded: HashMap::<Token, Box<ForwardedRequest>>::new(), //TODO: max forwards setting
            responses: Vec::<Response>::new(),
            params: params,
            server_token: token,
            last_request: Token(10), //Some number clearly different from the starting token
            pipeline: RequestPipeline::new()
        }
    }

    pub fn process(&mut self, mut request: &mut RawRequest, mut ctx: &mut RequestCtx) {
        if let Some(response) = self.pipeline.process(&mut request, ctx) {
            self.queue_response(ctx, response);            
            //reregister
        } else {
            //TODO: would rather this be in the pipeline...
            //No response, forward upstream
            let mut forward = self.build_forward_request(request.token, &request.bytes);
            debug!("Added {:?} to forwarded", forward.get().token);
            let response = forward.ready(&mut ctx);
            self.forwarded.insert(forward.get().token, forward);
        }
        // if request.state == forwarded {
        //     self.forwarded.push
        //     //reregister
        // }
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
        if let Some(cache_entry) = CacheEntry::from(&response.msg) {
            ctx.cache.write().unwrap().upsert(cache_entry.key.clone(), cache_entry);    
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
        //TODO: timeout is for forwarded requests...
        debug!("Timeout for {:?} {:?}", ctx.token, ctx.events);
        if let Some(mut req) = self.forwarded.get_mut(&ctx.token) {
            req.get_mut().on_timeout(ctx.token);
        }
    }

    pub fn owns(&self, token: Token) -> bool {
        //todo: self.pipeline.owns(forward stage)
        self.forwarded.contains_key(&token)
    }

        pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        debug!("Request for {:?} {:?}", ctx.token, ctx.events);
        let mut opt_response = None;
        //let mut opt_response = None;
        if let Some(ref mut request) = self.forwarded.get_mut(&ctx.token) {
            opt_response = request.ready(ctx);
        }
        if opt_response.is_some() {
            self.forwarded.remove(&ctx.token);
            self.queue_response(&ctx, opt_response.unwrap());
            //TODO: Not here
            // let boxed_req = self.forwarded.remove(&ctx.token).unwrap();
            // let base = boxed_req.get();
            // match opt_response {
            //     Some(ref x) => {
            //         let msg = DnsMessage::parse(x);       
            //         let response = Response::new(ctx.token, x.clone(), msg);
            //         self.queue_response(ctx, response); 
            //     }, None =>{}
            // }            
        }
        // let mut req = self.requests.remove(ctx.token).unwrap();
        // let response = self.pipeline.process(req.get_mut(), ctx).unwrap();
        // self.responses.push(response);
    }

    pub fn next_token(&mut self) -> Token {
        self.last_request = Token(self.last_request.as_usize() + 1);
        debug!("next_token gave -> {:?}", self.last_request);
        self.last_request
    }
}
