use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use mio::util::{Slab};
use server_mio::{MioServer,RequestCtx};
use request::base::*;
use cache::*;
//use std::net::SocketAddr;
use dns::dns_entities::*;

trait PipelineStage {
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response>;
}

struct RequestPipeline {
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
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response> {
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
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response> {        
        //TODO: parse should be Result. If it fails, we shoudl return a fail response here

        request.query = Some(DnsMessage::parse(&request.query_buf));
        debug!("Parsed query");
        None
    }
}

impl PipelineStage for AuthorityStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response> {        
        debug!("No Master File parsing yet, so no authoritative records");
        None
    }
}

impl PipelineStage for CacheStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Entered cahce stage");
        match ctx.cache.read() {
            Ok(cache) => {
                let query = DnsMessage::parse(&request.query_buf);
                let key = CacheKey::from(&query.question);
                if let Some(entry) = cache.get(&key) {
                    request.state = RequestState::ResponseFromCache;
                    //TODO: need to adjust the TTL down?
                    //TODO: cache the whole message?
                    let mut answer_header = query.header.clone();
                    answer_header.id = query.header.id;
                    answer_header.qr = true;
                    answer_header.ancount = entry.answers.len() as u16;
                    let msg = DnsMessage::new_reply(answer_header, query.question, entry.answers.clone());
                    debug!("Could answer with {:?} based on key {:?}", msg, entry.key);
                    let bytes = msg.to_bytes();
                    return Some(Response::new(ctx.token, msg, bytes));
                } 
                return None;
            }
            Err(e) => error!("Couldn't get read lock {:?}", e)
        }
        debug!("No cache hit");
        None
    }
}

impl PipelineStage for ForwardStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RequestBase, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Forward does nothing");
        None 
    }
}

pub struct ServerBase {
    //TODO: Slab is fixed size?
    pub request_factory: Box<RequestFactory>,
    pub requests: Slab<Box<Request>>,
    pub responses: Vec<Response>,
    pub params: RequestParams,
    server_token: Token,
    pipeline: RequestPipeline
}

impl ServerBase {
    pub fn new(requests: Slab<Box<Request>>, factory: Box<RequestFactory>, params: RequestParams, token: Token) -> ServerBase {
        ServerBase {
            requests: requests,
            request_factory: factory,
            responses: Vec::<Response>::new(),
            params: params,
            server_token: token,
            pipeline: RequestPipeline::new()
        }
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
        if let Some(req) = self.requests.remove(ctx.token) {
            debug!("parsing response...");
            let msg = DnsMessage::parse(&req.get().response_buf.as_ref().unwrap());
            debug!("{:?}", msg);
            //TODO: TTL must be same for all answers? Or the min?
            if let Some(cache_entry) = CacheEntry::from(&msg) {
                ctx.cache.write().unwrap().upsert(cache_entry.key.clone(), cache_entry);    
            }            
            self.responses.push(response);
            debug!("queued response {:?}", ctx.token);
        }
    }

    pub fn build_request(&mut self, token: Token, bytes: &[u8]) -> Box<Request> {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = RequestBase::new(token, buf, self.params);

        self.request_factory.new_with(request)
    }

    pub fn timeout(&mut self, ctx: &mut RequestCtx) {
        debug!("Timeout for {:?} {:?}", ctx.token, ctx.events);
        if let Some(mut req) = self.requests.get_mut(ctx.token) {
            req.get_mut().on_timeout(ctx.token);
        }
    }

    pub fn owns(&self, token: Token) -> bool {
        self.requests.contains(token)
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        debug!("Request for {:?} {:?}", ctx.token, ctx.events);
        let mut queue_response = false;
        let mut opt_response = None;
        if let Some(mut request) = self.requests.get_mut(ctx.token) {
            match self.pipeline.process(request.get_mut(), ctx) {
                Some(response) => {
                    opt_response = Some(response);
                    queue_response = true;
                },
                None => request.ready(ctx)
            }
        }
        if queue_response {
            self.queue_response(ctx, opt_response.unwrap());
        }
        // let mut req = self.requests.remove(ctx.token).unwrap();
        // let response = self.pipeline.process(req.get_mut(), ctx).unwrap();
        // self.responses.push(response);
    }
}
