use std::net::SocketAddr;
use mio::{Token, TryRead};
use mio::util::Slab;
use mio::tcp::{TcpStream,TcpListener};
use server_mio::{MioServer,RequestContext};
use std::collections::HashMap;
use request::base::*;
use request::tcp::TcpRequest;

pub struct TcpServer {
    pub server_socket: TcpListener,
    requests: Slab<TcpRequest>,
    responses: Vec<TcpRequest>,
    pending: HashMap<Token, TcpStream>,
    accepted: HashMap<Token, TcpStream>,
}
impl TcpServer {
    pub const TCP_SERVER_TOKEN: Token = Token(0);

    pub fn new(addr: SocketAddr, start_token: usize, max_connections: usize, params: RequestParams) -> TcpServer {
        let listener = Self::bind_tcp(addr);
        let requests = Slab::new_starting_at(Token(start_token), max_connections);
        TcpServer {
            server_socket: listener,
            requests: requests,
            responses: Vec::<TcpRequest>::new(),
            pending: HashMap::<Token, TcpStream>::new(),
            accepted: HashMap::<Token, TcpStream>::new(),
        }
    }

    pub fn bind_tcp(address: SocketAddr) -> TcpListener {
        info!("Binding TCP to {:?}", address);
        let server = TcpListener::bind(&address).unwrap();
        return server;
    }

    pub fn server_ready(&mut self, ctx: &mut RequestContext)  {

        //if self.pendng contains token
           //accept

        //otherwise...

        // if ctx.events.is_readable() {
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

    pub fn request_ready(&mut self, ctx: &mut RequestContext) -> bool {
        false
    }

    pub fn owns(&self, token: Token) -> bool {
        false
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
