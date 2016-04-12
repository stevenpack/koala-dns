use server_mio::{RequestCtx};
use request::base::*;
use cache::*;
use dns::dns_entities::*;

pub trait PipelineStage {
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response>;
}

pub struct RequestPipeline {
    stages: Vec<Box<PipelineStage>>
}

struct ParseStage;
struct AuthorityStage;
struct CacheStage;
struct ForwardStage;

impl RequestPipeline {
    pub fn new() -> RequestPipeline {
        
        let mut stages = Vec::<Box<PipelineStage>>::new();
        stages.push(Box::new(ParseStage));
        stages.push(Box::new(AuthorityStage));
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
        debug!("No Master File parsing yet, so no authoritative records");
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
