//#![allow(dead_code)]
use bit_cursor::BitCursor;


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
    id: u16,
    qr: bool,
    opcode: u8,
    aa: bool,
    tc: bool,
    rd: bool,
    ra: bool,
    z: u8,
    rcode: u8,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16
}

#[derive(Debug)]
pub struct DnsQuestion {
    header: DnsHeader
}

#[derive(Debug)]
pub struct DnsParser;

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

        let mut i = 0;
        let mut cursor = BitCursor::new();
        //iterate over each 16bit word in the packet
        for word in packet {
            //read each bit according to the definition
            cursor.set(word);
            println!("word: {:016b}", word);
            match i {
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
                5 => {
                    arcount = cursor.next_u16();
                    break;
                },
                _ => warn!("Trying to read past end of header")
            }
            i += 1;
        }

        return DnsHeader {
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

//or impl Iterator for WordIterator, impl Iterator for OctetIterator
//todo: new (english) word. Hextet, for 16 bytes.
impl<'a> Iterator for DnsPacket<'a> {
    type Item = u16;

    /*
    Returns two octets in the order they expressed in the spec. I.e. first byte shifted to the left
    */
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
    use super::{DnsParser, DnsPacket};

    fn test_buf() -> Vec<u8> {
        /*
         00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
         00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
         00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        */
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    #[test]
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

    #[test]
    fn iterate() {
        let buf = test_buf();
        let packet = DnsPacket::new(&buf);
        for word in packet {
            println!("word: {:016b}", word);
        }
    }
}
