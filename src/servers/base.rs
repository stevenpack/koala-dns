use mio::{EventLoop, EventSet, Token, PollOpt, Evented};
use mio::util::{Slab};
use server_mio::{MioServer,RequestContext};
use request::base::*;
use std::net::SocketAddr;

//ServerMixin
pub struct ServerBase<T> where T : IRequest<T> {
    pub requests: Slab<T>,
    pub responses: Vec<T>,
    pub params: RequestParams
}

impl<T> ServerBase<T> where T: IRequest<T> {
    pub fn new(requests: Slab<T>, responses: Vec<T>, params: RequestParams) -> ServerBase<T> {
        ServerBase {
            requests: requests,
            responses: responses,
            params: params
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

    pub fn queue_response(&mut self, token: Token) {
        self.requests.remove(token).and_then(|req| Some(self.responses.push(req)));
        debug!("queued {:?}", token);
    }

    pub fn build_request(&mut self, token: Token, addr: SocketAddr, bytes: &[u8]) -> T {
        let mut buf = Vec::<u8>::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        let request = RequestBase::new(token, buf, self.params);

        T::new_with(addr, request)
    }

    pub fn timeout(&mut self, ctx: &mut RequestContext) {
        self.requests.get_mut(ctx.token).unwrap().get_mut().on_timeout(ctx.token);
    }

    // fn register(&self,
    //         event_loop: &mut EventLoop<MioServer>,
    //         socket: &Evented,
    //         events: EventSet,
    //         token: Token,
    //         reregister: bool) {
    //     ServerBase::<T>::register(event_loop, socket, events, token, reregister);
    // }

    pub fn owns(&self, token: Token) -> bool {
        self.requests.contains(token)
    }
}
