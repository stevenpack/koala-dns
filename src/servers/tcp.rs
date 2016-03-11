use std::net::SocketAddr;
use server_mio::{RequestCtx};
use mio::{EventSet, Token, TryRead};
use mio::util::Slab;
use mio::tcp::{TcpStream,TcpListener};
use std::collections::HashMap;
use request::base::*;
use request::tcp::TcpRequest;
use servers::base::*;
use std::sync::{Arc, RwLock};

pub struct TcpServer {
    pub server_socket: TcpListener,
    pub base: ServerBase<TcpRequest>,
    pending: HashMap<Token, TcpStream>,
    accepted: HashMap<Token, TcpStream>,
}

impl TcpServer {
    pub const TCP_SERVER_TOKEN: Token = Token(0);

    pub fn new(addr: SocketAddr, start_token: usize, max_connections: usize, params: RequestParams, cache: Arc<RwLock<Cache>>) -> TcpServer {
        let listener = Self::bind_tcp(addr);
        let requests = Slab::new_starting_at(Token(start_token), max_connections);
        let responses = Vec::<TcpRequest>::new();
        TcpServer {
            server_socket: listener,
            pending: HashMap::<Token, TcpStream>::new(),
            accepted: HashMap::<Token, TcpStream>::new(),
            base: ServerBase::new(requests, responses, params, Self::TCP_SERVER_TOKEN, cache),
        }
    }

    pub fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }

    pub fn server_ready(&mut self, ctx: &mut RequestCtx)  {
        if ctx.events.is_readable() {
            self.accept(ctx);
        }
        self.base.reregister_server(ctx.event_loop, &self.server_socket, EventSet::readable());
    }

    pub fn request_ready(&mut self, ctx: &mut RequestCtx) {
        if self.pending.contains_key(&ctx.token) {
            self.accept_pending(ctx);
            return;
        }
        self.base.request_ready(ctx);

        if self.base.responses.len() > 0 {
            self.send_all(ctx);
        }
    }

    pub fn accept(&mut self, ctx: &mut RequestCtx) {
        match self.server_socket.accept() {
            Ok(Some((stream, addr))) => {
                    debug!("Accepted tcp request from {:?}. Now pending...", addr);
                    //request gets created with server token, and then updated with the slab token
                    let req = self.base.build_request(ctx.token, addr, Vec::<u8>::new().as_slice());
                    match self.base.requests.insert_with(|_| req) {
                        Some(token) => {
                            self.update_token(ctx.token, token);
                            self.base.register(ctx.event_loop, &stream, EventSet::readable(), token, false);
                            self.pending.insert(token, stream);
                        },
                        None => error!("Failed to insert request {:?}", ctx.token)
                    }
            }
            Ok(None) => debug!("Socket would block. Waiting..."),
            Err(err) => error!("Failed to accept tcp connection {}", err),
        }
    }

    fn update_token(&mut self, server_token: Token, client_token: Token) {
        match self.base.requests.get_mut(server_token) {
            Some(req) => req.get_mut().token = client_token,
            None => error!("No request waiting for updated token")
        }
    }

    fn accept_pending(&mut self, ctx: &mut RequestCtx) {
        debug_assert!(ctx.events.is_readable());
        match self.pending.remove(&ctx.token) {
            Some(mut stream) => {
                let buf = Self::receive_tcp(&mut stream);
                self.accepted.insert(ctx.token, stream);
                debug!("tcp accepted {:?}", ctx.token);
                match self.base.requests.get_mut(ctx.token) {
                    Some(request) => {
                        request.get_mut().query_buf = buf;
                        request.ready(ctx);
                    }
                    None => error!("Request {:?} not found", ctx.token),
                }
            }
            None => error!("{:?} was not pending", ctx.token),
        }
    }

    pub fn receive_tcp(stream: &mut TcpStream) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(512);
        match stream.try_read_buf(&mut buf) {
            Ok(Some(0)) => info!("Read 0 bytes"),
            Ok(Some(n)) => buf.truncate(n),
            Ok(None) => info!("None"),
            Err(err) => error!("read failed {}", err),
        }

        debug!("Read {} bytes", buf.len());
        //tcp has 2-byte lenth prefix
        return buf.split_off(2);
    }

    fn send_all(&mut self, ctx: &mut RequestCtx) {
        debug!("There are {} responses to send", self.base.responses.len());
        self.base.responses.pop().and_then(|reply| {
            let tok = reply.get().token;
            debug!("Will send {:?}", tok);
            match self.accepted.get_mut(&ctx.token) {
                Some(stream) => Some(reply.send(stream)),
                None => None
            }
        });
    }
}
