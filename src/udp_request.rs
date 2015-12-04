extern crate bytes;
use mio::{Token, EventSet, Timeout};
use mio::udp::UdpSocket;
use std::net::SocketAddr;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RequestState {
    New,
    Accepted,
    Forwarded,
    ResponseReceived
}

//
// Encapsulates the components of a Resolution request and response over Udp.
//
//#[derive(Debug)]
pub struct UdpRequest {
    pub state: RequestState,
    client_addr: SocketAddr,
    //upstream_token: Token,
    pub upstream_socket: UdpSocket,
    upstream_addr: SocketAddr,
    query_buf: Vec<u8>,
    response_buf: Vec<u8>,
    pub timeout_ms: u64,
    pub timeout_handle: Option<Timeout>
}


impl UdpRequest {
    pub fn new(client_addr: SocketAddr, upstream_addr: SocketAddr, query_buf: Vec<u8>, timeout: u64) -> UdpRequest {
        //debug!("New UDP transaction: {:?}", upstream_token);
        return UdpRequest {
            state: RequestState::New,
            client_addr: client_addr,
            //upstream_token: upstream_token,
            //todo: handle this by Option<> and error! the request but do not panic, or accepting in ctor
            upstream_socket: UdpSocket::v4().unwrap_or_else(|e| panic!("Failed to create UDP socket {:?}", e)),
            upstream_addr: upstream_addr,
            query_buf: query_buf,
            response_buf: Vec::<u8>::new(),
            timeout_ms: timeout,
            timeout_handle: None
        };
    }

    fn set_state(&mut self, state: RequestState) {
        debug!("{:?} -> {:?}", self.state, state);
        self.state = state;
    }

    pub fn set_timeout(&mut self, timeout: Timeout) {
        self.timeout_handle = Some(timeout);
    }

    pub fn socket_ready(&mut self, token: Token, events: EventSet) {
        debug!("Socket Event. State: {:?} {:?} EventSet: {:?}", self.state, token, events);
        //todo: refactor
        match self.state {
            RequestState::New => {
                assert!(events.is_readable());
                self.set_state(RequestState::Accepted);
            },
            RequestState::Accepted => {
                assert!(events.is_writable());
                match self.upstream_socket.send_to(&mut self::bytes::SliceBuf::wrap(self.query_buf.as_slice()), &self.upstream_addr) {
                    Ok(Some(_)) => {
                        self.set_state(RequestState::Forwarded);

                    }
                    Ok(None) => debug!("Failed to send. Expect writable event to fire again still in the same state. {:?}", token),
                    Err(e) => error!("Failed to write to upstream_socket. {:?}. {:?}", token, e),
                }
            },
            RequestState::Forwarded => {
                assert!(events.is_readable());
                //todo: higher perf buffer?
                let mut buf = Vec::<u8>::new();
                match self.upstream_socket.recv_from(&mut buf) {
                    Ok(Some(addr)) => {
                        debug!("Received {} bytes from {:?}", buf.len(), addr);
                        trace!("{:?}", buf);
                        self.response_buf = buf;
                        self.set_state(RequestState::ResponseReceived);

                    }
                    Ok(None) => debug!("No data received on upstream_socket. {:?}", token),
                    Err(e) => println!("Receive failed on {:?}. {:?}", token, e),
                }
            },
            _ => error!("Unexpected socket event for {:?}", token)
        }
    }

    pub fn send(&self, socket: &UdpSocket) {
        match socket.send_to(&mut self::bytes::SliceBuf::wrap(&self.response_buf.as_slice()), &self.client_addr) {
            Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, self.client_addr),
            Err(e) => error!("Failed to send. {:?} Error was {:?}", self.client_addr, e),
        }
    }
}
