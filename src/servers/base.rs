use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use mio::util::{Slab};
use server_mio::{MioServer,RequestCtx};
use request::base::*;
use servers::cache::*;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use dns::dns_entities::*;

pub struct ServerBase<T> where T : Request<T> {
    pub requests: Slab<T>,
    pub responses: Vec<T>,
    pub params: RequestParams,
    server_token: Token,
    cache: Arc<RwLock<ResolverCache>>
}

impl<T> ServerBase<T> where T: Request<T> {
    pub fn new(requests: Slab<T>, responses: Vec<T>, params: RequestParams, token: Token, cache: Arc<RwLock<ResolverCache>>) -> ServerBase<T> {
        ServerBase {
            requests: requests,
            responses: responses,
            params: params,
            server_token: token,
            cache: cache
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
        self.requests.remove(token).and_then(|req| {
            let msg = DnsMessage::parse(&req.get().response_buf.as_ref().unwrap());
            debug!("{:?}", msg);
            self.cache.write().unwrap().base.add(DnsKey::empty(), msg.answers[0].clone());
            debug!("cached it!");
            return Some(self.responses.push(req))
        });
        debug!("queued {:?}", token);
    }

    pub fn build_request(&mut self, token: Token, addr: SocketAddr, bytes: &[u8]) -> T {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = RequestBase::new(token, buf, self.params);

        T::new_with(addr, request)
    }

    pub fn timeout(&mut self, ctx: &mut RequestCtx) {
        debug!("Timeout for {:?}", ctx.token);
        self.requests.get_mut(ctx.token).unwrap().get_mut().on_timeout(ctx.token);
    }

    pub fn owns(&self, token: Token) -> bool {
        self.requests.contains(token)
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        debug!("request ready {:?}", ctx.token);

        let mut queue_response = false;
        match self.requests.get_mut(ctx.token) {
            Some(mut request) => {

                debug!("Maybe we don't need to forward? Here's the cache: {:?}", self.cache);

                request.ready(ctx);
                queue_response = request.get().has_reply();
            }
            None => error!("{:?} got routed to wrong server", ctx.token),
        }
        if queue_response {
            self.queue_response(ctx.token);
        }
    }
}
