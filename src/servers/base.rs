use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use mio::util::{Slab};
use server_mio::{MioServer,RequestCtx};
use request::base::*;
use cache::*;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use dns::dns_entities::*;

// trait IRequestProcessor {
//     fn process(&self, request: &mut RequestBase) -> Option<String>;
// }

// struct RequestParser;
// impl IRequestProcessor for RequestParser {
//     fn process(&self, request: &mut RequestBase) -> Option<String> {
        
//         None
//     }
// }


// struct RequestPipeline {
//     stages: Vec<Box<IRequestProcessor>>
// }

// impl RequestPipeline {
//     fn new() -> RequestPipeline {
//         RequestPipeline {
//             stages: Vec::<Box<IRequestProcessor>>::new()
//         }
//     }
// }

pub struct ServerBase<T> where T : Request<T> {
    //TODO: Slab is fixed size?
    pub requests: Slab<T>,
    pub responses: Vec<T>,
    pub params: RequestParams,
    server_token: Token,
    cache: Arc<RwLock<Cache>>,
    //pipeline: RequestPipeline
}

impl<T> ServerBase<T> where T: Request<T> {
    pub fn new(requests: Slab<T>, responses: Vec<T>, params: RequestParams, token: Token, cache: Arc<RwLock<Cache>>) -> ServerBase<T> {
        ServerBase {
            requests: requests,
            responses: responses,
            params: params,
            server_token: token,
            cache: cache,
            //pipeline: RequestPipeline::new()
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

    pub fn queue_response(&mut self, token: Token) {
        if let Some(req) = self.requests.remove(token) {
            let msg = DnsMessage::parse(&req.get().response_buf.as_ref().unwrap());
            debug!("{:?}", msg);
            //TODO: TTL must be same for all answers? Or the min?
            if let Some(cache_entry) = CacheEntry::from(&msg) {
                self.cache.write().unwrap().upsert(cache_entry.key.clone(), cache_entry);    
            }            
            self.responses.push(req);
            debug!("queued response {:?}", token);
        }
        
    }

    pub fn build_request(&mut self, token: Token, addr: SocketAddr, bytes: &[u8]) -> T {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = RequestBase::new(token, buf, self.params);

        T::new_with(addr, request)
    }

    pub fn timeout(&mut self, ctx: &mut RequestCtx) {
        debug!("Timeout for {:?} {:?}", ctx.token, ctx.events);
        self.requests.get_mut(ctx.token).unwrap().get_mut().on_timeout(ctx.token);
    }

    pub fn owns(&self, token: Token) -> bool {
        self.requests.contains(token)
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        debug!("Request for {:?} {:?}", ctx.token, ctx.events);

        //for each request processor
        //  process

        let mut queue_response = false;
        if let Some(mut request) = self.requests.get_mut(ctx.token) {
            request.ready_cache(ctx, self.cache.clone());
            queue_response = request.get().has_reply();
        }
        if queue_response {
            self.queue_response(ctx.token);
        }
    }
}
