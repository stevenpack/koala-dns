use std::net::SocketAddr;
use std::io::Write;
use server_mio::{RequestCtx};
use mio::{EventSet, Token, TryRead};
use mio::tcp::{TcpStream,TcpListener};
use std::collections::HashMap;
use request::base::*;
use request::tcp::{TcpRequestFactory};
use servers::base::*;

pub struct TcpServer {
    pub server_socket: TcpListener,
    pub base: ServerBase,
    pending: HashMap<Token, TcpStream>,
    accepted: HashMap<Token, TcpStream>,
}

impl TcpServer {
    pub const TCP_SERVER_TOKEN: Token = Token(0);

    pub fn new(addr: SocketAddr, max_connections: usize, params: RequestParams) -> TcpServer {
        let listener = Self::bind_tcp(addr);
        let factory = Box::new(TcpRequestFactory);
        TcpServer {
            server_socket: listener,
            pending: HashMap::<Token, TcpStream>::new(),
            accepted: HashMap::<Token, TcpStream>::new(),
            base: ServerBase::new(factory, params, Self::TCP_SERVER_TOKEN, max_connections),
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
        self.send_all();
    }

    pub fn accept(&mut self, ctx: &mut RequestCtx) {
        match self.server_socket.accept() {
            Ok(Some((stream, addr))) => {
                    debug!("Accepted tcp request from {:?}. Now pending...", addr);
                    let token = self.base.next_token();                    
                    self.base.register(ctx.event_loop, &stream, EventSet::readable(), token, false);    
                    self.pending.insert(token, stream);
            }
            Ok(None) => debug!("Socket would block. Waiting..."),
            Err(err) => error!("Failed to accept tcp connection {}", err),
        }
    }

    fn accept_pending(&mut self, ctx: &mut RequestCtx) {
        match self.pending.remove(&ctx.token) {
            Some(mut stream) => {
                let bytes = Self::receive_tcp(&mut stream);
                self.accepted.insert(ctx.token, stream);
                let mut request = RawRequest::new(ctx.token, bytes);
                self.base.process(&mut request, ctx);
                debug!("tcp accepted {:?}", ctx.token);               
                //TODO: send now? or register as writable. favour fast response or throughput?
                self.send_all();
            }
            None => error!("{:?} was not pending", ctx.token),
        }
    }

    pub fn receive_tcp(stream: &mut TcpStream) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(4096);
        match stream.try_read_buf(&mut buf) {
            Ok(Some(0)) => warn!("Read 0 bytes"),
            Ok(Some(n)) => buf.truncate(n),
            Ok(None) => info!("None"),
            Err(err) => error!("read failed {}", err),
        }
        debug!("Read {} bytes", buf.len());
        //tcp has 2-byte lenth prefix
        return buf.split_off(2);
    }

   pub fn owns(&self, token: Token) -> bool {
        self.pending.contains_key(&token) || self.base.owns(token)
    }

    fn prefix_with_length(buf: &mut Vec<u8>) {
        //TCP responses are prefixed with a 2-byte length
        let len = buf.len() as u8;
        buf.insert(0, len);
        buf.insert(0, 0);
        debug!("Added 2b prefix of len: {:?}", len);
    }

     fn send_all(&mut self) {
        debug!("There are {} tcp responses to send", self.base.responses.len());
        while self.base.responses.len() > 0 {
            if let Some(reply) = self.base.responses.pop() {
                debug!("Will send {:?}", reply.token);
                if let Some(stream) = self.accepted.get_mut(&reply.token) {
                    Self::send(reply, stream)
                }
            }
        }
    }

    pub fn send(response: Response, socket: &mut TcpStream) {
        debug!("{:?} bytes in response", response.bytes.len());
        let mut prefixed_response = response.bytes;
        Self::prefix_with_length(&mut prefixed_response);
        match socket.write(&mut prefixed_response.as_slice()) {
            Ok(n) => debug!("{:?} bytes sent to client. {:?}", n, socket.peer_addr()),
            Err(e) => error!("Failed to send. {:?} Error was {:?}", socket.peer_addr(), e),
        }
    }
}
