//#![allow(dead_code)]
use bit_cursor::BitCursor;
use dns_packet::*;
use dns_entities::*;

#[derive(Debug)]
pub struct DnsParser;

impl DnsParser {

    pub fn parse(buf: &[u8]) -> DnsQuery  {
        let (header, pos) = DnsParser::parse_header(buf);
        match header.qr {
            QR_QUERY => {let _ = DnsParser::parse_question(buf, pos);},
            QR_RESPONSE => info!("parse answer")
        }
        return DnsQuery {
            header: header
        };
    }

    fn parse_question(buf: &[u8], pos: usize) -> DnsQuestion {
        let packet = DnsPacket::new_at(buf, pos);
        let mut cursor = BitCursor::new();
        println!("{:?}", packet);
        println!("Resuming at {}", pos);
        for (word, pos) in packet {
            cursor.set(word);
            println!("{:016b}", word);
            let length = cursor.next_u8();
            println!("string length is: {}", length);
            //first octet is length, then that many octets for each part.
            //i.e. yahoo.com is
            //00000101 y
            //a        h
            //o        o
            //00000011 c
            //o        m
            //00000000
            //terminated with 00000000
            //todo: rfc says no padding, but shows qtype aligning at start of next word.
            //test with with odd number of octets
        }
        return DnsQuestion {
            qname: format!("{}", "test"),
            qtype: 0,
            qclass: 1
        }
    }

    ///Parses the buffer, returning a DnsHeader and the octet at which to continue.
    fn parse_header(buf: &[u8]) -> (DnsHeader, usize) {
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

        let packet = DnsPacket::new(buf);
        let mut cursor = BitCursor::new();
        //iterate over each 16bit word in the packet
        let mut last_pos = 0;
        for (word, pos) in packet {
            last_pos = pos;
            //read each bit according to the definition
            cursor.set(word);
            trace!("word: {:016b}", word);
            match pos {
                0 => id = cursor.next_u16(),
                1 => {
                    qr = cursor.next_bool();
                    opcode = cursor.next_u4();
                    aa = cursor.next_bool();
                    tc = cursor.next_bool();
                    rd = cursor.next_bool();
                    ra = cursor.next_bool();
                    z = cursor.next_u4();
                    rcode = cursor.next_u4();
                }
                2 => qdcount = cursor.next_u16(),
                3 => ancount = cursor.next_u16(),
                4 => nscount = cursor.next_u16(),
                5 => arcount = cursor.next_u16(),
                _ => break
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
        //We were iterating 16bit words, but we return the octet
        //todo: hack with position
        return (header, last_pos + 2);
    }
}

#[cfg(test)]
mod tests {
    use super::DnsParser;
    use dns_packet::DnsPacket;

    fn test_buf() -> Vec<u8> {
        /*
         00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
         00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
         00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        */
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    fn dump(buf: &Vec<u8>) {
        let packet = DnsPacket::new_at(buf, 0);
        for word in packet {
            println!("{:016b}", word.0);
        }
    }

    #[test]
    #[ignore]
    fn parse_header (){
        println!("{:?}", DnsParser::parse_header(&test_buf()));
    }

    #[test]
    fn parse_question (){
        dump(&test_buf());
        println!("{:?}", DnsParser::parse_question(&test_buf(), 12));
    }

    #[test]
    #[ignore]
    fn parse() {
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
        let buf = test_buf();
        let q = DnsParser::parse(&buf);
        println!("{:?}", q);
    }
}
