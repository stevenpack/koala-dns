use server_mio::{RequestCtx};
use request::base::*;
use cache::*;
use dns::message::*;

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

impl Default for RequestPipeline {
    fn default() -> RequestPipeline {
        
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
        for stage in &self.stages {
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
        self.get_authoritive(request)
    }
}

impl AuthorityStage {

    fn get_authoritive(&self, request: &RawRequest) -> Option<Response> {
        
        if let Some(ref query) = request.query {
            if let Some(question) = query.first_question() {
                debug!("Checking for authoritive answer to {:?}", question.qname);
                if question.qname.to_string() == "example.org"  {                    
                    let msg = self.test_answer(&query.header, &question);
                    debug!("Yes. Will answer with authoritive answer. {:?}", msg);
                    return Some(Response::with_source(request.token, msg.to_bytes(), msg, Source::Authoritive));
                }
            }
        }
        None
    }

    fn test_answer(&self, query_header: &DnsHeader, question: &DnsQuestion) -> DnsMessage {
        let mut answer_header = query_header.clone();
        answer_header.id = query_header.id;
        answer_header.qr = true;
        answer_header.ancount = 1;
        answer_header.aa = true; //authoritive
        answer_header.ra = true;

        let mut answers = Vec::<DnsAnswer>::new();
        let answer = DnsAnswer {
            name: question.qname.clone(),
            aclass: 1, //A
            atype: 1,  //IN(ternet)
            ttl: 300,
            rdlength: 4,
            rdata: vec![93, 184, 216, 34]
        };
        answers.push(answer);
        DnsMessage::new_reply(answer_header, vec![question.clone()], answers)        
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

                        if entry.calc_ttl() == 0 {
                            //Expired. Will be removed on next upsert
                            debug!("expired");
                            return None;
                        }

                        //TODO: cache the whole message?
                        let mut answer_header = query.header.clone();
                        answer_header.id = query.header.id;
                        answer_header.qr = true;
                        answer_header.ra = true;
                        answer_header.ancount = entry.answers.len() as u16;
                        let mut answers = entry.answers.clone();
                        Self::adjust_ttl(entry.calc_ttl(), &mut answers);
                        let msg = DnsMessage::new_reply(answer_header, query.questions.clone(), answers);
                        debug!("Will answer with {:?} based on key {:?}", msg, entry.key);
                        return Some(Response::with_source(ctx.token, msg.to_bytes(), msg, Source::Cache));
                    } 
                }
            }
            Err(e) => error!("Couldn't get read lock {:?}", e)
        }
        debug!("No cache hit");
        None
    }
}

impl CacheStage {
    fn adjust_ttl(ttl: u32, answers: &mut Vec<DnsAnswer>) {
        for answer in answers {
            debug!("Adjusting ttl {} -> {}", answer.ttl, ttl);
            answer.ttl = ttl;
        }
    }
}

impl PipelineStage for ForwardStage {
    #[allow(unused_variables)]
    fn process(&self, request: &mut RawRequest, ctx: &RequestCtx) -> Option<Response> {        
        debug!("No cache or authoritive answer. Create ForwardRequest from RequestRaw...");
        None 
    }
}
