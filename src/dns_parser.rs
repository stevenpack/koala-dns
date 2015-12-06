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

        let a: u16 = buf[0] as u16;
        let id: u16 = a << 8 | buf[1] as u16;
        println!("id: {:?}", id);

        return DnsHeader {
            id: 1,
            qr: true,
            op_code: 5
        }
    }
}
#[cfg(test)]
mod tests {
    //use super::*;
    use super::DnsParser;
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
        let buf: [u8; 27] = [8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
        let q = DnsParser::parse(&buf);
        println!("{:?}", q);
    }

    fn bits() {
        //let a: bool
    }
}
