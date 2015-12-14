//#![allow(dead_code)]
use bit_cursor::BitCursor;
use dns_packet::*;
use dns_entities::*;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub struct DnsParser;

impl DnsParser {

    pub fn parse(buf: &[u8]) -> DnsMessage  {
        let mut packet = DnsPacket::new(buf);
        let header = DnsParser::parse_header(&mut packet);
        match header.qr {
            QR_QUERY => {
                let questions = DnsParser::parse_questions(&mut packet, header.qdcount);
                return DnsMessage::new_query(header, questions);
            },
            QR_RESPONSE => {
                let questions = DnsParser::parse_questions(&mut packet, header.qdcount);
                let answers = DnsParser::parse_answers(&mut packet, header.ancount);
                return DnsMessage::new_reply(header, questions, answers);
            }
        }
    }

    fn parse_answer(packet: &mut DnsPacket) -> DnsAnswer {
        let name = DnsParser::parse_name(packet);
        let atype = packet.next_u16().unwrap_or_default();
        let aclass = packet.next_u16().unwrap_or_default();
        let ttl = packet.next_u32().unwrap_or_default();
        let rdlength = packet.next_u16().unwrap_or_default();
        //todo: if parsing rdata fails for some reason, we should fail, or make sure we start
        //reading the next answer at the right position
        let rdata = packet.next_bytes(rdlength as usize);
        return DnsAnswer::new(name, atype, aclass, ttl, rdlength, rdata);
    }

    fn parse_answers(packet: &mut DnsPacket, ancount: u16) -> Vec<DnsAnswer> {
        let mut answers = Vec::<DnsAnswer>::with_capacity(ancount as usize);
        for _ in 0..ancount {
            let answer = DnsParser::parse_answer(packet);
            answers.push(answer);
        }
        println!("{:?}", answers);
        return answers;
    }

    fn parse_label(packet: &mut DnsPacket, len: usize) -> Result<String, FromUtf8Error> {
        let mut label = Vec::<u8>::with_capacity(len as usize);
        for i in 0..len {
            match packet.next_u8() {
                Some(0) | None => break,
                Some(byte) => label.insert(i, byte)
            }
        }
        return String::from_utf8(label);
    }

    fn parse_name(packet: &mut DnsPacket) -> String {
        let mut more_labels = true;

        //todo: size to remaining words or some better estimate like packet.word_count()
        if DnsParser::is_pointer(packet.peek_u8().unwrap_or_default()) {
            let offset = DnsParser::parse_pointer(packet.next_u16().unwrap_or_default());
            //todo: refactor and test with part pointers. i.e, only part of the name has pointers
            //see example page 30
            let current_pos = packet.pos();
            packet.seek(offset as usize);
            let name = DnsParser::parse_name(packet);
            packet.seek(current_pos);
            return name;
        } else {
            let mut labels = Vec::<String>::with_capacity(8);
            while more_labels {
                match packet.next_u8() {
                    //terminated with 00000000
                    Some(0) | None => more_labels = false,
                    Some(len) => {
                        match DnsParser::parse_label(packet, len as usize) {
                                Ok(label) => labels.push(label),
                                Err(e) => warn!("Invalid label: {}", e)
                            };
                    }
                }
            }
            return labels.join(".");
        }
    }

    fn is_pointer(byte: u8) -> bool {
        //DNS message compression 4.1.4
        return byte & 0b1100_0000 == 0b1100_0000
    }

    fn parse_pointer(byte: u16) -> u16 {
        return byte & 0b0011_1111_1111_1111;
    }

    fn parse_question(packet: &mut DnsPacket) -> DnsQuestion {
        let qname = DnsParser::parse_name(packet);
        let qtype = packet.next_u16().unwrap_or_default();
        let qclass = packet.next_u16().unwrap_or_default();

        let question = DnsQuestion::new(qname, qtype, qclass);
        return question;
    }

    fn parse_questions(packet: &mut DnsPacket, qdcount: u16) -> Vec<DnsQuestion> {
        //todo: test with multiple questions
        let mut questions = Vec::<DnsQuestion>::with_capacity(qdcount as usize);
        for _ in 0..qdcount {
            //todo: rfc says no padding, but shows qtype aligning at start of next word.
            //test with with odd number of octets
            let question = DnsParser::parse_question(packet);
            questions.push(question);
        }
        return questions;
    }

    fn parse_header(packet: &mut DnsPacket) -> DnsHeader {
        //todo: see bitflags macro
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
        //iterate over each 16bit word in the packet
        for (word, pos) in packet {
            //read each bit according to the definition
            cursor.set(word);
            //println!("word: {:016b}", word);
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
                },
                x => error!("Read past end of header. Pos: {:?}", x)
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
                        arcount: arcount
                    };
        return header;
    }
}

#[cfg(test)]
mod tests {
    use super::DnsParser;
    use dns_packet::DnsPacket;

    fn test_query_buf() -> Vec<u8> {
        /*
         00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
         00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
         00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        */
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    fn test_reply_buf() -> Vec<u8> {
        return vec![8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36, 45, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 138, 253, 109];
    }

    #[test]
    //#[ignore]
    fn parse_reply() {
        println!("{:?}", DnsParser::parse(&test_reply_buf()));
    }

    #[test]
    #[ignore]
    fn parse_header (){
        //println!("{:?}", DnsParser::parse_header(&test_buf()));
    }

    #[test]
    #[ignore]
    fn parse_query() {
        //query
        //
        //[8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0,
        // 1, 0, 1]
        //
        //reply
        //
        //[8, 113, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0,
        // 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 206, 190, 36, 45, 192, 12, 0, 1, 0,
        // 1, 0, 0, 0, 10, 0, 4, 98, 139, 183, 24, 192, 12, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 98, 138
        //, 253, 109]
        //
        //dig yahoo.com @127.0.0.1 -p 10001
        //; <<>> DiG 9.8.3-P1 <<>> yahoo.com @127.0.0.1 -p 10001
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
        let q = DnsParser::parse(&test_query_buf());
        println!("{:?}", q);
    }
}
