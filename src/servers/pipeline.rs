use server_mio::{RequestCtx};
use request::base::*;
use cache::*;
use dns::dns_entities::*;
use authority::*;

pub trait PipelineStage {
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response>;
}

pub struct RequestPipeline {
    stages: Vec<Box<PipelineStage>>
}

struct ParseStage;
//TODO: MasterFile per thread, or shared? Depends on size. For a resolver with millions of records, shared. For a few, per thread.
struct AuthorityStage {
    master: Master
}
struct CacheStage;
struct ForwardStage;

impl RequestPipeline {
    pub fn new() -> RequestPipeline {
        
        let mut stages = Vec::<Box<PipelineStage>>::new();
        stages.push(Box::new(ParseStage));
        stages.push(Box::new(AuthorityStage::new()));
        stages.push(Box::new(CacheStage));
        stages.push(Box::new(ForwardStage));
        
        RequestPipeline {
            stages: stages
        }
    }
}

impl PipelineStage for RequestPipeline {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {
        for stage in self.stages.iter() {
            if let Some(response) = stage.process(request, ctx) {
                return Some(response)
            }
        }
        None
    }
}

impl PipelineStage for ParseStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        //TODO: DnsMessage::parse should be Result. If it fails, we shoudl return a fail response here
        request.query = Some(DnsMessage::parse(&request.bytes));
        debug!("Parsed query");
        None
    }
}

impl PipelineStage for AuthorityStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Checking master file...");
        self.get_authoritive(request)
    }
}

impl AuthorityStage {

    fn new() -> AuthorityStage {
        let mut master_file = MasterFile::new(String::from("/tmp/some_master_file.txt"));
        let master = master_file.create();
        AuthorityStage {
            master: master
        }
    }

    fn get_authoritive(&self, request: &RawRequest) -> Option<Response> {
        
        //TODO: Hack, the message shoud already be parsed and ready to use
        let query_msg = DnsMessage::parse(&request.bytes);
        let query = query_msg.first_question().unwrap();
        let key = RecordKey {
            name: query.qname.to_string(),
            typex: query.qtype,
            class: query.qclass

        };
        if let Some(record) = self.master.get(&key) {
            debug!("AUTHORITIVE ANSWER!!! {:?}", record);
            let mut answer_header = query_msg.header.clone();
            answer_header.id = query_msg.header.id;
            answer_header.qr = true;
            answer_header.ancount = 1;

            let mut answers = Vec::<DnsAnswer>::new();
            let answer = DnsAnswer {
                name: query.qname.clone(),
                aclass: record.class,
                atype: record.typex,
                ttl: record.ttl,
                rdlength: 4,
                rdata: vec![10, 10, 10, 10]
            };
            answers.push(answer);

            let msg = DnsMessage::new_reply(answer_header, query_msg.questions.clone(), answers);
            return Some(Response {
                token: request.token,
                bytes: msg.to_bytes(),
                msg: msg
            });
        }
        None
    }
}

impl PipelineStage for CacheStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Entered cache stage");
        match ctx.cache.read() {
            Ok(cache) => {
                let query = DnsMessage::parse(&request.bytes);
                if let Some(question) = query.first_question() {
                    let key = CacheKey::from(&question);
                    if let Some(entry) = cache.get(&key) {
                        //TODO: need to adjust the TTL down?
                        //TODO: cache the whole message?
                        let mut answer_header = query.header.clone();
                        answer_header.id = query.header.id;
                        answer_header.qr = true;
                        answer_header.ancount = entry.answers.len() as u16;
                        let msg = DnsMessage::new_reply(answer_header, query.questions.clone(), entry.answers.clone());
                        debug!("Will answer with {:?} based on key {:?}", msg, entry.key);
                        return Some(Response::new(ctx.token, msg.to_bytes(), msg));
                    } 
                }
            }
            Err(e) => error!("Couldn't get read lock {:?}", e)
        }
        debug!("No cache hit");
        None
    }
}

impl PipelineStage for ForwardStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("Forward does nothing. Create ForwardRequest from RequestRaw...");
        None 
    }
}
