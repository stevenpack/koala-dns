
//                               1  1  1  1  1  1
// 0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                      ID                       |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |QR|   Opcode  |AA|TC|RD|RA|   Z    |   RCODE   |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                    QDCOUNT                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                    ANCOUNT                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                    NSCOUNT                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                    ARCOUNT                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
#[derive(Debug)]
pub struct DnsHeader {
    pub id: u16,
    pub qr: bool,
    pub opcode: u8,
    pub aa: bool,
    pub tc: bool,
    pub rd: bool,
    pub ra: bool,
    pub z: u8,
    pub rcode: u8,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16
}

#[derive(Debug)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsAnswer>,
    pub msg_type: DnsMessageType
}

#[derive(Debug)]
pub enum DnsMessageType {
    Query,
    Reply
}

impl DnsMessage {
    pub fn new_query(header: DnsHeader, questions: Vec<DnsQuestion>) -> DnsMessage {
        return DnsMessage::new(header, questions, vec![], DnsMessageType::Query);
    }

    pub fn new_reply(header: DnsHeader, questions: Vec<DnsQuestion>, answers: Vec<DnsAnswer>) -> DnsMessage {
        return DnsMessage::new(header, questions, answers, DnsMessageType::Reply);
    }
    pub fn new(header: DnsHeader, questions: Vec<DnsQuestion>, answers: Vec<DnsAnswer>, msg_type: DnsMessageType) -> DnsMessage {
        return DnsMessage {
            header: header,
            questions: questions,
            answers: answers,
            msg_type: msg_type
        }
    }
}

#[derive(Debug)]
pub struct DnsAnswer {
    pub name: String,
    pub atype: u16,
    pub aclass: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: Vec<u8>
}

impl DnsAnswer {
    pub fn new(name: String, atype: u16, aclass: u16, ttl: u32, rdlength: u16, rdata: Vec<u8>) -> DnsAnswer {
        return DnsAnswer {
            name: name,
            atype: atype,
            aclass: aclass,
            ttl: ttl,
            rdlength: rdlength,
            rdata: rdata
        }
    }
}

#[derive(Debug)]
pub struct DnsQuestion {
    pub qname: String,
    pub qtype: u16,
    pub qclass: u16
}

impl DnsQuestion {
    pub fn new(qname: String, qtype: u16, qclass: u16) -> DnsQuestion {
        return DnsQuestion {
            qname: qname,
            qtype: qtype,
            qclass: qclass
        }
    }
}

pub const QR_QUERY: bool = false;
pub const QR_RESPONSE: bool = true;
