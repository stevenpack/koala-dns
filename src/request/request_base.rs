use std::net::SocketAddr;
use mio::{Token, Timeout, Handler, EventSet, Evented, PollOpt};
use server_mio::RequestContext;
use dns::dns_entities::DnsMessage;
use dns::dns_entities::DnsHeader;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived,
    Error,
}

pub struct RequestBase {
    pub state: RequestState,
    pub query_buf: Vec<u8>,
    pub response_buf: Option<Vec<u8>>,
    pub timeout_handle: Option<Timeout>,

    // Separate? This is only required for upstream
    pub params: RequestParams,
}

pub struct RequestParams {
    pub timeout: u64,
    pub upstream_addr: SocketAddr,
}

impl RequestBase {
    pub fn new(query_buf: Vec<u8>, params: RequestParams) -> RequestBase {
        return RequestBase {
            state: RequestState::New,
            query_buf: query_buf,
            response_buf: None,
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

    pub fn set_timeout(&mut self, ctx: &mut RequestContext) {
        match ctx.event_loop.timeout_ms(ctx.token, self.params.timeout) {
            Ok(t) => self.set_timeout_handle(t),
            Err(e) => error!("Failed to schedule timeout for {:?}. {:?}", ctx.token, e),
        }
    }

    pub fn clear_timeout(&mut self, ctx: &mut RequestContext) {
        match self.timeout_handle {
            Some(handle) => {
                if ctx.event_loop.clear_timeout(handle) {
                    debug!("Timeout cleared for {:?}", ctx.token);
                } else {
                    warn!("Could not clear timeout for {:?}", ctx.token);
                }
            }
            None => warn!("Timeout handle not present"),
        }
    }

    pub fn register_upstream(&mut self,
                             ctx: &mut RequestContext,
                             events: EventSet,
                             sock: &Evented) {

        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        ctx.event_loop
           .register(sock, ctx.token, events, poll_opt)
           .unwrap_or_else(|e| {
               self.error_with(format!("Failed to register upstream socket. {}", e))
           });
    }

    pub fn buffer_response(&mut self, buf: &[u8], count: usize) {
        let mut response = Vec::with_capacity(count);
        response.extend_from_slice(&buf);
        response.truncate(count);
        self.response_buf = Some(response);
    }

    pub fn error_with(&mut self, err_msg: String) {
        self.set_state(RequestState::Error);
        info!("{}", err_msg);
        let req = DnsMessage::parse(&self.query_buf);
        let reply = DnsHeader::new_error(req.header, 2);
        let vec = reply.to_bytes();
        self.response_buf = Some(vec);
    }

    pub fn has_reply(&self) -> bool {
        return self.response_buf.is_some();
    }
}
