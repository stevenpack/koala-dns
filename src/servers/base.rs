use mio::{EventLoop, EventSet, Token, TryRead, PollOpt, Evented};
use server_mio::{MioServer,RequestContext};
pub struct ServerBase;

impl ServerBase {
    pub fn register(event_loop: &mut EventLoop<MioServer>,
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
}

pub trait Server {
    //fn get(&self) -> ServerBase;
    fn register(&self,
            event_loop: &mut EventLoop<MioServer>,
            socket: &Evented,
            events: EventSet,
            token: Token,
            reregister: bool) {
        ServerBase::register(event_loop, socket, events, token, reregister);
    }
}
