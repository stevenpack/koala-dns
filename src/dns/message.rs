use dns::bit_cursor::BitCursor;
use dns::dns_packet::DnsPacket;
use dns::mut_dns_packet::MutDnsPacket;
use buf::*;
use std::iter;
use std::str::FromStr;

//note: qdcount doesn't really make sense and most dns servers don't respect it. How do you
//correlate the multiple answers to multiple questions? what do the flags apply to?

#[derive(Debug)]
#[derive(Clone)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsAnswer>,
    pub msg_type: DnsMessageType,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Eq)]
#[derive(PartialEq)]
pub enum DnsMessageType {
    Query,
    Reply,
}

#[derive(Debug)]
#[derive(Clone)]
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
    pub arcount: u16,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Eq)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
pub struct DnsAnswer {
    pub name: DnsName,
    pub atype: u16,
    pub aclass: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: Vec<u8>,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct DnsQuestion {
    pub qname: DnsName,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Eq)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
pub struct DnsName {
    labels: Vec<String>
}

impl IntoBytes for DnsMessage {

    fn write(&self, mut packet: &mut MutDnsPacket) -> usize {
        
        let mut pos = self.header.write(packet);

        //Replies also have the question in the answer msg
        for question in &self.questions {
            pos = question.write(packet);
        }            
        if self.msg_type == DnsMessageType::Reply {
            //TODO: apply outbound compression
            for answer in &self.answers {
                pos = answer.write(packet);
            }    
        }        
        pos
    }
}

impl DnsHeader {
    #[allow(similar_names)]
    pub fn new_error(request_header: DnsHeader, rcode: u8) -> DnsHeader {
        DnsHeader {
            id: request_header.id,
            qr: true,
            opcode: request_header.opcode,
            aa: request_header.aa,
            tc: false, // todo. was the message truncated?
            rd: request_header.rd,
            ra: true,
            z: 0,
            rcode: rcode,
            qdcount: 0,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }   

    #[allow(similar_names)]
    fn parse(packet: &mut DnsPacket) -> DnsHeader {
        let mut id: u16 = 0;
        let mut qr: bool = false;
        let mut opcode: u8 = 0;
        let mut aa: bool = false;
        let mut tc: bool = false;
        let mut rd: bool = false;
        let mut ra: bool = false;
        let mut z: u8 = 0;
        let mut rcode: u8 = 0;

        let mut qdcount: u16 = 0;
        let mut ancount: u16 = 0;
        let mut nscount: u16 = 0;
        let mut arcount: u16 = 0;

        let mut cursor = BitCursor::default();
        // iterate over each 16bit word in the packet
        for (word, pos) in packet {
            // read each bit according to the definition
            cursor.set(word);
            trace!("word: {:016b}", word);
            match pos {
                2 => id = cursor.next_u16(),
                4 => {
                    qr = cursor.next_bool();
                    opcode = cursor.next_u4();
                    aa = cursor.next_bool();
                    tc = cursor.next_bool();
                    rd = cursor.next_bool();
                    ra = cursor.next_bool();
                    z = cursor.next_u4();
                    rcode = cursor.next_u4();
                }
                6 => qdcount = cursor.next_u16(),
                8 => ancount = cursor.next_u16(),
                10 => nscount = cursor.next_u16(),
                12 => {
                    arcount = cursor.next_u16();
                    trace!("Breaking on end of header at {:?}", pos);
                    break;
                }
                x => error!("Read past end of header. Pos: {:?}", x),
            }
        }

        DnsHeader {
            id: id,
            qr: qr,
            opcode: opcode,
            aa: aa,
            tc: tc,
            rd: rd,
            ra: ra,
            z: z,
            rcode: rcode,
            qdcount: qdcount,
            ancount: ancount,
            nscount: nscount,
            arcount: arcount,
        }
    }
}

impl IntoBytes for DnsHeader {

    fn write(&self, mut packet: &mut MutDnsPacket) -> usize {
        packet.write_u16(self.id); //1st word of header
        if let Some(val) = packet.next_u16() {
            let mut bit_cursor = BitCursor::new_with(val);
            bit_cursor.write_bool(self.qr); //qr
            bit_cursor.write_u4(self.opcode); //opcode
            bit_cursor.write_bool(self.aa); //aa
            bit_cursor.write_bool(self.tc); //tc
            bit_cursor.write_bool(self.rd); //rd
            bit_cursor.write_bool(self.ra); //ra
            bit_cursor.write_u4(self.z); //z
            bit_cursor.write_u4(self.rcode); //rcode
            bit_cursor.seek(0);
            packet.seek(2);
            packet.write_u16(bit_cursor.next_u16()); //2nd word of header
            packet.write_u16(self.qdcount); //qdcount
            packet.write_u16(self.ancount); //ancount
            packet.write_u16(self.nscount); //nscount
            packet.write_u16(self.arcount); //arcount
        }
        debug!("{:?} bytes in header. self={:?}", packet.pos(), self);        
        packet.pos()
    }
}

impl DnsMessage {
    pub fn parse(buf: &[u8]) -> DnsMessage {
        //TODO: return Result<T,DnsParseError> for parsing, rather than using unwrap_or_default().
        //      msg is either valid, or it's not
        let mut packet = DnsPacket::new(buf);
        let header = DnsHeader::parse(&mut packet);
        if header.qr {
            //answer
            let questions = Self::parse_questions(&mut packet, header.qdcount);
            let answers = Self::parse_answers(&mut packet, header.ancount);
            Self::new_reply(header, questions, answers)
        } else {
            let questions = Self::parse_questions(&mut packet, header.qdcount);
            Self::new_query(header, questions)
        }
    }

    pub fn new_error(header: DnsHeader) -> DnsMessage {
        Self::new(header, vec![], vec![], DnsMessageType::Reply)
    }

    fn new_query(header: DnsHeader, questions: Vec<DnsQuestion>) -> DnsMessage {
        Self::new(header, questions, vec![], DnsMessageType::Query)
    }

    pub fn new_reply(header: DnsHeader, questions: Vec<DnsQuestion>, answers: Vec<DnsAnswer>) -> DnsMessage {
        Self::new(header, questions, answers, DnsMessageType::Reply)
    }

    fn new(header: DnsHeader,
           questions: Vec<DnsQuestion>,
           answers: Vec<DnsAnswer>,
           msg_type: DnsMessageType)
           -> DnsMessage {
        DnsMessage {
            header: header,
            questions: questions,
            answers: answers,
            msg_type: msg_type,
        }
    }

    fn parse_questions(packet: &mut DnsPacket, qdcount: u16) -> Vec<DnsQuestion> {
        if qdcount > 1 {
            warn!("Invalid qdcount {:?} only 0 or 1 is valid. Ignoring other questions", qdcount);
        }
        let mut questions = Vec::with_capacity(qdcount as usize);
        for _ in 0..qdcount {
            questions.push(DnsQuestion::parse(packet));
        }
        questions
    }

    fn parse_answers(packet: &mut DnsPacket, ancount: u16) -> Vec<DnsAnswer> {
        let mut answers = Vec::<DnsAnswer>::with_capacity(ancount as usize);
        for _ in 0..ancount {
            let answer = DnsAnswer::parse(packet);
            answers.push(answer);
        }
        answers
    }

    pub fn first_question(&self) -> Option<&DnsQuestion> {
        self.questions.get(0)
    }

    pub fn first_answer(&self) -> Option<&DnsAnswer> {
        self.answers.get(0)
    }
}

impl DnsAnswer {
    pub fn new(name: DnsName,
           atype: u16,
           aclass: u16,
           ttl: u32,
           rdlength: u16,
           rdata: Vec<u8>)
           -> DnsAnswer {
        DnsAnswer {
            name: name,
            atype: atype,
            aclass: aclass,
            ttl: ttl,
            rdlength: rdlength,
            rdata: rdata,
        }
    }   

    fn parse(packet: &mut DnsPacket) -> DnsAnswer {
        let name = DnsName::parse(packet);
        let atype = packet.next_u16().unwrap_or_default();
        let aclass = packet.next_u16().unwrap_or_default();
        let ttl = packet.next_u32().unwrap_or_default();
        let rdlength = packet.next_u16().unwrap_or_default();
        let rdata = packet.next_bytes(rdlength as usize);
        Self::new(name, atype, aclass, ttl, rdlength, rdata)
    }
}

impl IntoBytes for DnsAnswer {

    fn write(&self, mut packet: &mut MutDnsPacket) -> usize {
        self.name.write(packet);
        packet.write_u16(self.atype);
        packet.write_u16(self.aclass);
        packet.write_u32(self.ttl);
        packet.write_u16(self.rdlength);
        packet.write_bytes(&self.rdata.clone());
        debug!("{:?} bytes in answer", packet.pos());        
        packet.pos()
    }
}

impl IntoBytes for DnsQuestion {

    fn write(&self, mut packet: &mut MutDnsPacket) -> usize {
        self.qname.write(packet);
        packet.write_u16(self.qtype);
        packet.write_u16(self.qclass);
        debug!("{:?} bytes in question", packet.pos());        
        packet.pos()
    }
}

impl DnsQuestion {
    fn new(qname: DnsName, qtype: u16, qclass: u16) -> DnsQuestion {
        DnsQuestion {
            qname: qname,
            qtype: qtype,
            qclass: qclass,
        }
    }

    fn parse(packet: &mut DnsPacket) -> DnsQuestion {
        let qname = DnsName::parse(packet);
        let qtype = packet.next_u16().unwrap_or_default();
        let qclass = packet.next_u16().unwrap_or_default();
        DnsQuestion::new(qname, qtype, qclass)
    }
}

impl FromStr for DnsName {
    type Err=String;
     fn from_str(string: &str) -> Result<Self, Self::Err> {        
        Ok(Self::from_string(string.to_owned()))
    }
}

impl DnsName {

   
    pub fn from_string(string: String) -> DnsName {
        let labels = string.split('.').map(|s| s.to_owned()).collect();
        Self::from(labels)
    }

    fn from(labels: Vec<String>) -> DnsName {
        DnsName {
            labels: labels
        }
    }

    pub fn to_string(&self) -> String {
        self.labels.join(".")
    }

    ///A series of labels separatd by dots
    // labels may be actual labels, or pointers to previous instances of labels
    fn parse(packet: &mut DnsPacket) -> DnsName {
        let byte = packet.peek_u8().unwrap_or_default();
        if Self::is_pointer(byte) {
            return Self::parse_pointer(packet);
        } 
        let labels = Self::parse_labels(packet);
        DnsName::from(labels)        
    }

    fn parse_labels(packet: &mut DnsPacket) -> Vec<String> {
        let mut labels = Vec::<String>::with_capacity(8);
        let mut more_labels = true;
        while more_labels {
            match packet.next_u8() {
                // terminated with 00000000
                Some(0) | None => more_labels = false,
                Some(len) => {
                    match Self::parse_label(packet, len as usize) {
                        Ok(label) => labels.push(label),
                        Err(e) => warn!("Invalid label: {}", e),
                    };
                }
            }
        }
        labels
    }

    ///A length octet followed by that many octets as string characters
    fn parse_label(packet: &mut DnsPacket, len: usize) -> Result<String, String> {
        let mut label = Vec::<u8>::with_capacity(len as usize);
        for i in 0..len {
            match packet.next_u8() {
                Some(0) | None => {
                    return Err(format!("Found terminating byte or end of buffer before len ({}) bytes read",
                          len));
                }
                Some(byte) => label.insert(i, byte),
            }
        }
        trace!("label bytes {:?}", label);
        let label_str = String::from_utf8(label);
        trace!("label: {:?}", label_str);
        label_str.map_err( |err| format!("Label to UTF8 parse failure {:?}", err))
    }

    fn is_pointer(byte: u8) -> bool {
        // DNS message compression 4.1.4
        byte & 0b1100_0000 == 0b1100_0000
    }

    fn parse_offset(byte: u16) -> u16 {
        byte & 0b0011_1111_1111_1111
    }

    fn parse_pointer(packet: &mut DnsPacket) -> DnsName {
        let offset = Self::parse_offset(packet.next_u16().unwrap_or_default());
        let current_pos = packet.pos();
        if packet.seek(offset as usize) {
            let name = Self::parse(packet);
            packet.seek(current_pos);
            return name;
        }
        warn!("Invalid offset {:?}", offset);
        DnsName::from(Vec::<String>::new())
    }
}

impl IntoBytes for DnsName {

    fn write(&self, mut packet: &mut MutDnsPacket) -> usize {
       
        for label in &self.labels {
            packet.write_u8(label.len() as u8);
            let label_bytes = label.clone().into_bytes();            
            packet.write_bytes(&label_bytes);
        }
        //terminate
        packet.write_u8(0);
        //TODO: outbound compression (we do it in parsing)
        packet.pos()
    }
}

pub trait IntoBytes {
    fn to_bytes(&self) -> Vec<u8> {
        //a zero'd buffer so the len() checks see enough room
        let mut buf = iter::repeat(0).take(4096).collect::<Vec<_>>();
        let byte_count;
        {
            let mut packet = MutDnsPacket::new(&mut buf);
            byte_count = self.write(&mut packet);
            debug!("{:?} bytes from to_bytes()", byte_count);
        }
        buf.truncate(byte_count);
        buf
    }
    fn write(&self, mut packet: &mut MutDnsPacket) -> usize;
}


#[cfg(test)]
mod tests {
    use super::*;
    use buf::*;
    use test::Bencher;

    #[test]
    fn to_bytes() {
       let msg = DnsMessage::parse(&test_query_buf());
       println!("bytes: {:?}", msg.to_bytes());
    }

    #[test]
    fn round_trip() {
        let mut query = test_query_buf();
        let msg = DnsMessage::parse(&query);
        let mut query_out = msg.header.to_bytes();
        query.split_off(12);
        query_out.split_off(12);
        //compare the headers
        assert_eq!(query, query_out);
        
    }

    fn test_query_buf() -> Vec<u8> {
        //
        // 00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
        // 00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
        // 00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        //
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111,
                    109, 0, 0, 1, 0, 1];
    }


    fn test_reply_buf() -> Vec<u8> {
        return vec![8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99,
                    111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36,
                    45, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1,
                    0, 1, 0, 0, 0, 10, 0, 4, 98, 138, 253, 109];
    }

    #[test]
    fn parse_reply() {
        let reply = DnsMessage::parse(&test_reply_buf());
        println!("{:?}", reply);
        assert_eq!(2161, reply.header.id);
        // todo: test more flags
        assert_eq!(1, reply.header.qdcount);
        assert_eq!(1, reply.questions.len());
        assert_eq!(3, reply.header.ancount);
        assert_eq!(3, reply.answers.len());

        let a = &reply.answers[0];
        assert_eq!("yahoo.com", a.name.to_string());
        assert_eq!(10, a.ttl);
        assert_eq!(4, a.rdlength);
        assert_eq!(vec![206, 190, 36, 45], a.rdata);
    }

    #[test]
    fn parse_query() {
        // query
        //
        // [8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0,
        // 1, 0, 1]
        //
        // reply
        //
        // [8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0,
        // 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36, 45, 192, 12, 0, 1, 0,
        // 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 138
        // , 253, 109]
        //
        // dig yahoo.com @127.0.0.1 -p 10001
        // ; <<>> DiG 9.8.3-P1 <<>> yahoo.com @127.0.0.1 -p 10001
        // ;; global options: +cmd
        // ;; Got answer:
        // ;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 2161
        // ;; flags: qr rd ra; QUERY: 1, ANSWER: 3, AUTHORITY: 0, ADDITIONAL: 0
        //
        // ;; QUESTION SECTION:
        // ;yahoo.com.			IN	A
        //
        // ;; ANSWER SECTION:
        // yahoo.com.		10	IN	A	206.190.36.45
        // yahoo.com.		10	IN	A	98.139.183.24
        // yahoo.com.		10	IN	A	98.138.253.109
        //
        // ;; Query time: 112 msec
        // ;; SERVER: 127.0.0.1#10001(127.0.0.1)
        // ;; WHEN: Sat Dec  5 14:49:55 2015
        // ;; MSG SIZE  rcvd: 75
        let q = DnsMessage::parse(&test_query_buf());
        println!("{:?}", q);
        assert_eq!(2161, q.header.id);
        assert_eq!(1, q.header.qdcount);
        assert_eq!(1, q.questions.len());
        assert_eq!("yahoo.com", q.questions[0].qname.to_string());
    }

    // todo: test with multiple questions. We ignore... shoudl probably FORMATFAIL
    // todo: test with part pointers. i.e, only part of the name has pointers
    // see example page 30 of RFC1035


    #[bench]
    fn parse_query_bench(b: &mut Bencher) {
        let query = test_query_buf();
        b.iter(|| DnsMessage::parse(&query));
    }

    #[bench]
    fn parse_reply_bench(b: &mut Bencher) {
        let reply = test_reply_buf();
        b.iter(|| DnsMessage::parse(&reply));
    }
}
