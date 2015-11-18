pub struct Server {
    pub port: i32
}

pub trait Start {
    fn start(&self);
}

impl Start for Server {
    fn start(&self) {
        println!("Starting server on port {}", self.port);
    }
}
