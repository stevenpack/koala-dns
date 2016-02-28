use mio::Timeout;
use std::net::SocketAddr;

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
}
