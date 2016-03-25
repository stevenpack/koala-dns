use dns::bit_cursor::BitCursor;
use dns::dns_packet::DnsPacket;
use dns::mut_dns_packet::MutDnsPacket;
use buf::*;

//note: qdcount doesn't really make sense and most dns servers don't respect it. How do you
//correlate the multiple answers to multiple questions? what do the flags apply to?
//TODO: return Result<T,DnsParseError> for parsing

#[derive(Debug)]
#[derive(Clone)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub question: DnsQuestion,
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
    pub name: String,
    pub atype: u16,
    pub aclass: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: Vec<u8>,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct DnsQuestion {
    pub qname: String,
    pub qtype: u16,
    pub qclass: u16,
}

pub struct DnsName;

pub const QR_QUERY: bool = false;
pub const QR_RESPONSE: bool = true;

// #[derive(PartialEq)]
// #[derive(Debug)]
// pub enum OpCode {
//     Query=0,
//     IQuery=1,
//     Status=2
// }

impl IntoBytes for DnsMessage {

    fn write(&self, buf: &mut [u8]) -> usize {
        let mut byte_count = 0;
        byte_count += self.header.write(buf);
        byte_count += self.question.write(buf);
        if self.msg_type == DnsMessageType::Reply {
            for answer in self.answers.iter() {
                byte_count += answer.write(buf);
            }    
        }        
        byte_count
    }
}

impl DnsHeader {
    pub fn new_error(request_header: DnsHeader, rcode: u8) -> DnsHeader {
        let header = DnsHeader {
            id: request_header.id,
            qr: true,
            opcode: request_header.opcode,
            aa: request_header.aa,
            tc: false, // todo
            rd: request_header.rd,
            ra: request_header.ra,
            z: 0,
            rcode: rcode,
            qdcount: 0,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        };
        return header;
    }

   

    fn parse(packet: &mut DnsPacket) -> DnsHeader {
        // todo: see bitflags macro
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

        let mut cursor = BitCursor::new();
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

        let header = DnsHeader {
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
        };
        return header;
    }
}

impl IntoBytes for DnsHeader {

    fn write(&self, mut buf: &mut [u8]) -> usize {
        let mut packet = MutDnsPacket::new(&mut buf);
        let res = packet.write_u16(self.id); //1st word of header
        debug!("res: {:?}", res);        
        if let Some(val) = packet.next_u16() {
            let mut bit_cursor = BitCursor::new_with(val);
            bit_cursor.write_bool(true); //qr
            bit_cursor.write_u4(0); //opcode
            bit_cursor.write_bool(false); //aa
            bit_cursor.write_bool(false); //tc
            bit_cursor.write_bool(true); //rd
            bit_cursor.write_bool(true); //ra
            bit_cursor.write_u4(0); //z
            bit_cursor.write_u4(self.rcode); //rcode
            bit_cursor.seek(0);
            packet.seek(2);
            packet.write_u16(bit_cursor.next_u16()); //2nd word of header
            packet.write_u16(self.qdcount); //qdcount
            packet.write_u16(self.ancount); //ancount
            packet.write_u16(self.nscount); //nscount
            packet.write_u16(self.arcount); //arcount
        }
        debug!("{:?} bytes in header", packet.pos());        
        packet.pos()
    }
}

impl DnsMessage {
    pub fn parse(buf: &Vec<u8>) -> DnsMessage {
        let mut packet = DnsPacket::new(buf);
        let header = DnsHeader::parse(&mut packet);
        match header.qr {
            QR_QUERY => {
                let question = Self::parse_question(&mut packet, header.qdcount);
                return Self::new_query(header, question);
            }
            QR_RESPONSE => {
                let question = Self::parse_question(&mut packet, header.qdcount);
                let answers = Self::parse_answers(&mut packet, header.ancount);
                return Self::new_reply(header, question, answers);
            }
        }
    }

    fn new_query(header: DnsHeader, questions: DnsQuestion) -> DnsMessage {
        return Self::new(header, questions, vec![], DnsMessageType::Query);
    }

    pub fn new_reply(header: DnsHeader,
                 question: DnsQuestion,
                 answers: Vec<DnsAnswer>)
                 -> DnsMessage {
        return Self::new(header, question, answers, DnsMessageType::Reply);
    }

    fn new(header: DnsHeader,
           question: DnsQuestion,
           answers: Vec<DnsAnswer>,
           msg_type: DnsMessageType)
           -> DnsMessage {
        return DnsMessage {
            header: header,
            question: question,
            answers: answers,
            msg_type: msg_type,
        };
    }

    fn parse_question(packet: &mut DnsPacket, qdcount: u16) -> DnsQuestion {
        if qdcount != 1 {
            warn!("Invalid qdcount {:?} only 1 is valid. Ignoring other quesitons", qdcount);
        }
        DnsQuestion::parse(packet)
    }

    fn parse_answers(packet: &mut DnsPacket, ancount: u16) -> Vec<DnsAnswer> {
        let mut answers = Vec::<DnsAnswer>::with_capacity(ancount as usize);
        for _ in 0..ancount {
            let answer = DnsAnswer::parse(packet);
            answers.push(answer);
        }
        return answers;
    }

    pub fn first_answer(&self) -> Option<&DnsAnswer> {
        self.answers.get(0)
    }
}

impl DnsAnswer {
    pub fn new(name: String,
           atype: u16,
           aclass: u16,
           ttl: u32,
           rdlength: u16,
           rdata: Vec<u8>)
           -> DnsAnswer {
        return DnsAnswer {
            name: name,
            atype: atype,
            aclass: aclass,
            ttl: ttl,
            rdlength: rdlength,
            rdata: rdata,
        };
    }

   

    fn parse(packet: &mut DnsPacket) -> DnsAnswer {
        let name = DnsName::parse(packet);
        let atype = packet.next_u16().unwrap_or_default();
        let aclass = packet.next_u16().unwrap_or_default();
        let ttl = packet.next_u32().unwrap_or_default();
        let rdlength = packet.next_u16().unwrap_or_default();
        let rdata = packet.next_bytes(rdlength as usize);
        return Self::new(name, atype, aclass, ttl, rdlength, rdata);
    }
}

impl IntoBytes for DnsAnswer {

    fn write(&self, mut buf: &mut [u8]) -> usize {
        let mut packet = MutDnsPacket::new(&mut buf);
        packet.write_bytes(&DnsName::to_bytes(self.name.clone()));
        packet.write_u16(self.atype);
        packet.write_u16(self.aclass);
        packet.write_u16(self.rdlength);
        packet.write_bytes(&self.rdata.clone());
        debug!("{:?} bytes in answer", packet.pos());        
        packet.pos()
    }
}

impl IntoBytes for DnsQuestion {

    fn write(&self, mut buf: &mut [u8]) -> usize {
        let mut packet = MutDnsPacket::new(&mut buf);
        packet.write_bytes(&DnsName::to_bytes(self.qname.clone()));
        packet.write_u16(self.qtype);
        packet.write_u16(self.qclass);
        debug!("{:?} bytes in question", packet.pos());        
        packet.pos()
    }
}

impl DnsQuestion {
    fn new(qname: String, qtype: u16, qclass: u16) -> DnsQuestion {
        return DnsQuestion {
            qname: qname,
            qtype: qtype,
            qclass: qclass,
        };
    }

    fn parse(packet: &mut DnsPacket) -> DnsQuestion {
        let qname = DnsName::parse(packet);
        let qtype = packet.next_u16().unwrap_or_default();
        let qclass = packet.next_u16().unwrap_or_default();
        let question = DnsQuestion::new(qname, qtype, qclass);
        return question;
    }
}

impl DnsName {

    fn to_bytes(name: String) -> Vec<u8> {        
        let mut buf = Vec::<u8>::new(); //TODO: could come up with a good estimate here
        // buf.insert(0, name.len() as u)
        // buf.append(name.into_bytes())
        let labels: Vec<&str> = name.split(".").collect();
        for label in labels {
            buf.push(label.len() as u8);
            buf.append(&mut String::from(label).into_bytes());
        }
        //TODO compression (we do it in parsing)
        buf
    }
    ///A series of labels separatd by dots
    // labels may be actual labels, or pointers to previous instances of labels
    fn parse(packet: &mut DnsPacket) -> String {
        let byte = packet.peek_u8().unwrap_or_default();
        if Self::is_pointer(byte) {
            let name = Self::parse_pointer(packet);
            return name;
        } else {
            let labels = Self::parse_labels(packet);
            return labels.join(".");
        }
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
        return labels;
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
        return label_str.map_err( |err| format!("Label to UTF8 parse failure {:?}", err));
    }

    fn is_pointer(byte: u8) -> bool {
        // DNS message compression 4.1.4
        return byte & 0b1100_0000 == 0b1100_0000;
    }

    fn parse_offset(byte: u16) -> u16 {
        return byte & 0b0011_1111_1111_1111;
    }

    fn parse_pointer(packet: &mut DnsPacket) -> String {
        let offset = Self::parse_offset(packet.next_u16().unwrap_or_default());
        let current_pos = packet.pos();
        if packet.seek(offset as usize) {
            let name = Self::parse(packet);
            packet.seek(current_pos);
            return name;
        }
        warn!("Invalid offset {:?}", offset);
        return String::new();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use buf::*;
    //use test::Bencher;

    #[test]
    fn to_bytes() {
       let msg = DnsMessage::parse(&test_query_buf());
       println!("bytes: {:?}", msg.to_bytes());
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
}

//     fn test_reply_buf() -> Vec<u8> {
//         return vec![8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99,
//                     111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36,
//                     45, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1,
//                     0, 1, 0, 0, 0, 10, 0, 4, 98, 138, 253, 109];
//     }

//     #[test]
//     fn parse_reply() {
//         let reply = DnsMessage::parse(&test_reply_buf());
//         println!("{:?}", reply);
//         assert_eq!(2161, reply.header.id);
//         // todo: more flags
//         // todo: assert_eq!(0, OpCode::Query);
//         assert_eq!(1, reply.header.qdcount);
//         //assert_eq!(1, reply.questions.len());
//         assert_eq!(3, reply.header.ancount);
//         assert_eq!(3, reply.answers.len());

//         let ref a = reply.answers[0];
//         assert_eq!("yahoo.com", a.name);
//         assert_eq!(10, a.ttl);
//         assert_eq!(4, a.rdlength);
//         assert_eq!(vec![206, 190, 36, 45], a.rdata);
//         // todo: other answers

//     }

//     #[test]
//     fn parse_query() {
//         // query
//         //
//         // [8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0,
//         // 1, 0, 1]
//         //
//         // reply
//         //
//         // [8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0,
//         // 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36, 45, 192, 12, 0, 1, 0,
//         // 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 138
//         // , 253, 109]
//         //
//         // dig yahoo.com @127.0.0.1 -p 10001
//         // ; <<>> DiG 9.8.3-P1 <<>> yahoo.com @127.0.0.1 -p 10001
//         // ;; global options: +cmd
//         // ;; Got answer:
//         // ;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 2161
//         // ;; flags: qr rd ra; QUERY: 1, ANSWER: 3, AUTHORITY: 0, ADDITIONAL: 0
//         //
//         // ;; QUESTION SECTION:
//         // ;yahoo.com.			IN	A
//         //
//         // ;; ANSWER SECTION:
//         // yahoo.com.		10	IN	A	206.190.36.45
//         // yahoo.com.		10	IN	A	98.139.183.24
//         // yahoo.com.		10	IN	A	98.138.253.109
//         //
//         // ;; Query time: 112 msec
//         // ;; SERVER: 127.0.0.1#10001(127.0.0.1)
//         // ;; WHEN: Sat Dec  5 14:49:55 2015
//         // ;; MSG SIZE  rcvd: 75
//         let q = DnsMessage::parse(&test_query_buf());
//         println!("{:?}", q);
//         assert_eq!(2161, q.header.id);
//         // todo: assert_eq!(0, OpCode::Query);
//         assert_eq!(1, q.header.qdcount);
//         //assert_eq!(1, q.questions.len());
//         assert_eq!("yahoo.com", q.question.qname);
//         // todo: more flags
//     }

//     // todo: test with multiple questions
//     // todo: test with part pointers. i.e, only part of the name has pointers
//     // see example page 30


//     #[bench]
//     fn parse_query_bench(b: &mut Bencher) {
//         let query = test_query_buf();
//         let buf = query.as_slice();
//         b.iter(|| DnsMessage::parse(&buf));
//     }

//     #[bench]
//     fn parse_reply_bench(b: &mut Bencher) {
//         let reply = test_reply_buf();
//         let buf = reply.as_slice();
//         b.iter(|| DnsMessage::parse(&buf));
//     }
// }
