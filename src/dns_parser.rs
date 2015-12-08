#![allow(dead_code)]

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
#[allow(dead_code)]
struct DnsHeader {
    id: u16,    //0-15
    qr: bool,   //16-16
    op_code: u8, //17-20
}

#[derive(Debug)]
struct DnsQuestion {
    header: DnsHeader
}

#[derive(Debug)]
struct DnsParser;

impl DnsParser {
    pub fn parse(buf: &[u8]) -> DnsQuestion  {
        let header = DnsParser::parse_header(buf);
        return DnsQuestion {
            header: header
        };
    }

    fn parse_header(buf: &[u8]) -> DnsHeader {
        println!("{:?}", buf);
        let packet = DnsPacket::new(buf);

        let mut id: u16 = 0;
        let mut qr: bool = false;
        let mut op_code: u8 = 0;

        let mut i = 0;
        for word in packet {
            println!("word: {:016b}", word);
            match i {
                0 => id = word,
                1 => {
                    qr = word & 0b1000_0000_0000_0000 == 1;
                    op_code = (word & 0b0111_1000_0000_0000) as u8;
                    //or
                    // let bits = BitCursor::new(word);
                    // qr = bits.next_bool();
                    // op_code = bits.next_u4();
                    // aa = bits.next_bool();
                }
                _ => {
                    //parse
                }
            }
            i += 1;
        }

        return DnsHeader {
            id: id,
            qr: qr,
            op_code: op_code
        }
    }
}

struct DnsPacket<'a> {
    buf: &'a [u8],
    pos: usize
}

impl<'a> DnsPacket<'a> {
    fn new(buf: &[u8]) -> DnsPacket {
        return DnsPacket {
            buf: buf,
            pos: 0
        }
    }
}

impl<'a> Iterator for DnsPacket<'a> {
    //Or make this a BitCursor
    //and just keep re-using it
    //consumer can do that (and still re-use it)
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        let len = self.buf.len();
        if self.pos >= len {
            return None;
        }
        let byte1 = self.buf[self.pos];
        self.pos += 1;

        let mut byte2 = 0b0000_0000;
        if self.pos < len {
            byte2 = self.buf[self.pos];
            self.pos +=1;
        }
        return Some(((byte1 as u16) << 8) | byte2 as u16)
    }
 }
#[cfg(test)]
mod tests {
    //use super::*;
    use super::{DnsParser, DnsPacket};

    fn test_buf() -> Vec<u8> {
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    #[test]
    fn it_works() {
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

    #[test]
    fn iterate() {
        let buf = test_buf();
        let packet = DnsPacket::new(&buf);
        for word in packet {
            println!("word: {:016b}", word);
        }
    }
}
