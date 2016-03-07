use std::net::SocketAddr;
use mio::{EventLoop, EventSet, Token, TryRead};
use mio::util::Slab;
use mio::tcp::{TcpStream,TcpListener};
use server_mio::{MioServer,RequestContext};
use std::collections::HashMap;
use request::base::*;
use request::tcp::TcpRequest;
use servers::base::{ServerBase};

pub struct TcpServer {
    pub server_socket: TcpListener,
    //requests: Slab<TcpRequest>,
    //responses: Vec<TcpRequest>,
    pending: HashMap<Token, TcpStream>,
    accepted: HashMap<Token, TcpStream>,
    //params: RequestParams,
    pub base: ServerBase<TcpRequest>
}

impl TcpServer {
    pub const TCP_SERVER_TOKEN: Token = Token(0);

    pub fn new(addr: SocketAddr, start_token: usize, max_connections: usize, params: RequestParams) -> TcpServer {
        let listener = Self::bind_tcp(addr);
        let requests = Slab::new_starting_at(Token(start_token), max_connections);
        let responses = Vec::<TcpRequest>::new();
        TcpServer {
            server_socket: listener,
            pending: HashMap::<Token, TcpStream>::new(),
            accepted: HashMap::<Token, TcpStream>::new(),
            base: ServerBase::new(requests, responses, params)
        }
    }

    pub fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }

    pub fn server_ready(&mut self, ctx: &mut RequestContext)  {


        if ctx.events.is_readable() {
            if self.pending.contains_key(&ctx.token) {
                self.accept_pending(ctx)
            } else {
                self.accept(ctx);
            }
        }
        //     self.accept(&ctx)
        //         .and_then( |req| self.requests.insert(req).ok())
        //         .and_then( |tok| Some(RequestContext::new(ctx.event_loop, EventSet::readable(), tok)))
        //         .and_then( |req_ctx| Some((self.requests.get_mut(req_ctx.token), req_ctx)))
        //         .and_then( |(req, mut req_ctx)| Some(req.unwrap().ready(&mut req_ctx)));
        // }
        // if ctx.events.is_writable() {
        //     self.send_reply();
        // }
        // We are always listening for new requests. The server socket will be regregistered
        // as writable if there are responses to write
        //self.reregister_server(ctx.event_loop, EventSet::readable());
    }

    //TODO: trait
    pub fn request_ready(&mut self, ctx: &mut RequestContext) {
        debug!("request ready {:?}", ctx.token);

        let mut queue_response = false;
        match self.base.requests.get_mut(ctx.token) {
            Some(mut request) => {
                request.ready(ctx);
                queue_response = request.inner.has_reply();
            }
            None => {/* must be a tcp request*/},
        }
        if queue_response {
            self.base.queue_response(ctx.token);
            //self.reregister_server(ctx.event_loop, EventSet::readable() | EventSet::writable());
        }
    }

    //TODO: trait
    //TODO: so self.base.owns, or self.owns? is the trait defined to call down worth it?
    // pub fn owns(&self, token: Token) -> bool {
    //     //self.pending.contains_key(&token) || self.accepted.contains_key(&token)
    //     self.base.owns(token)
    // }

    pub fn accept(&mut self, ctx: &mut RequestContext) {
        match self.server_socket.accept() {
            Ok(Some((stream, addr))) => {
                    debug!("Accepted tcp request from {:?}", addr);
                    let req = self.base.build_request(addr, Vec::<u8>::new().as_slice());
                    let tok = self.base.requests.insert_with(|tok| req).unwrap();
                    self.base.register(ctx.event_loop, &stream, EventSet::readable(), tok, true);
                    self.pending.insert(tok, stream);
            }
            Ok(None) => debug!("Socket would block. Waiting..."),
            Err(err) => error!("Failed to accept tcp connection {}", err),
        }
    }

    fn accept_pending(&mut self, ctx: &mut RequestContext) {
        debug_assert!(ctx.events.is_readable());
        match self.pending.remove(&ctx.token) {
            Some(mut stream) => {
                let buf = Self::receive_tcp(&mut stream);
                self.accepted.insert(ctx.token, stream);
                debug!("tcp accepted {:?}", ctx.token);
                match self.base.requests.get_mut(ctx.token) {
                    Some(request) => {
                        request.inner.query_buf = buf;
                    }
                    None => error!("Request {:?} not found", ctx.token),
                }
            }
            None => error!("{:?} was not pending", ctx.token),
        }
    }

    pub fn receive_tcp(stream: &mut TcpStream) -> Vec<u8> {
        info!("Have a TcpStream to receive from to {:?}", stream);

        let mut buf = Vec::<u8>::with_capacity(512);
        match stream.try_read_buf(&mut buf) {
            Ok(Some(0)) => info!("Read 0 bytes"),
            Ok(Some(n)) => buf.truncate(n),
            Ok(None) => info!("None"),
            Err(err) => error!("read failed {}", err),
        }

        info!("Read {} bytes", buf.len());
        // TODO: FIRST TWO BYTES IN TCP ARE LENGTH
        let mut b2 = Vec::from(buf);
        let b3 = b2.split_off(2);
        // let msg = DnsMessage::parse(&b3);
        // debug!("{:?}", msg);
        return b3.clone();
    }
}
