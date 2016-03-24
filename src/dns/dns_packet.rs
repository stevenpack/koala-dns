use buf::*;

///Wrapper over a buffer providing seek, next, seek etc.
#[derive(Debug)]
pub struct DnsPacket<'a> {
    buf: &'a Vec<u8>,
    pos: usize,
}

impl<'a> DirectAccessBuf for DnsPacket<'a> {
    fn pos(&self) -> usize {
        return self.pos;
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn len(&self) -> usize {
        return self.buf().len();
    }
}
impl<'a> BufRead for DnsPacket<'a> {
    fn buf(&self) -> &Vec<u8> {
        return self.buf;
    }
}

impl<'a> DnsPacket<'a> {
    pub fn new(buf: &Vec<u8>) -> DnsPacket {
        return DnsPacket::new_at(buf, 0);
    }

    pub fn new_at(buf: &Vec<u8>, pos: usize) -> DnsPacket {
        return DnsPacket {
            buf: buf,
            pos: pos,
        };
    }
}

///Iterate each 16bit word in the packet
impl<'a> Iterator for DnsPacket<'a> {
    ///2 octets of data and the position
    type Item = (u16, usize);

    ///
    ///Returns two octets in the order they expressed in the spec. I.e. first byte shifted to the left
    ///
    fn next(&mut self) -> Option<(u16, usize)> {
        return self.next_u16().and_then(|n| return Some((n, self.pos)));
    }
}
// #[cfg(test)]
// mod tests {
//     use super::DnsPacket;
//     use buf::*;

//     fn test_buf() -> Vec<u8> {
//         //
//         // 00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
//         // 00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
//         // 00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
//         //
//         return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111,
//                     109, 0, 0, 1, 0, 1];
//     }

//     #[test]
//     fn next_u32() {
//         let buf = &test_buf();
//         let mut p = DnsPacket::new(buf);

//         // println!("u32: {:?}", p.next_u32());
//         assert_eq!(141623552, p.next_u32().unwrap());

//         let len = p.len();
//         // try read 4 bytes when we're 3 byte from the end
//         p.seek(len - 3);
//         assert_eq!(None, p.next_u32());
//         // now try again when we're two bytes away
//         p.seek(len - 4);
//         assert_eq!(true, p.next_u32().is_some());

//     }

//     #[test]
//     fn seek() {
//         let buf = &test_buf();
//         let mut p = DnsPacket::new(buf);
//         assert_eq!(true, p.seek(5));
//         assert_eq!(5, p.pos());
//         // can't go past end
//         assert_eq!(false, p.seek(1000));
//         // position unchanged
//         assert_eq!(5, p.pos());
//     }

//     #[test]
//     fn peek_u8() {
//         let buf = &test_buf();
//         let p = DnsPacket::new(buf);
//         assert_eq!(8, p.peek_u8().unwrap());
//         // don't move position for a peek
//         assert_eq!(0, p.pos());
//     }

//     #[test]
//     fn next_u8() {
//         let buf = &test_buf();
//         let mut p = DnsPacket::new(buf);
//         assert_eq!(8, p.next_u8().unwrap());
//         // move position for a peek
//         assert_eq!(1, p.pos());
//         // return none, don't panic at the end
//         let pos = p.len();
//         p.seek(pos);
//         assert_eq!(None, p.next_u8());
//     }

//     #[test]
//     fn next_u8_boundary() {
//         let buf = &test_buf();
//         let mut p = DnsPacket::new(buf);
//         let len = p.len();
//         // go to the end
//         assert_eq!(true, p.seek(len));
//         // can't read past the end
//         assert_eq!(None, p.next_u8());
//     }

//     #[test]
//     fn iterate() {
//         let buf = test_buf();
//         let packet = DnsPacket::new(&buf);
//         for word in packet {
//             println!("word: {:016b} {:?}", word.0, word.1);
//         }
//     }

//     #[test]
//     fn next_bytes() {
//         let buf = test_buf();
//         let mut p = DnsPacket::new(&buf);
//         // read some bytes
//         let vec = p.next_bytes(10);
//         assert_eq!(vec.len(), 10);
//         // read past the end
//         let vec2 = p.next_bytes(100);
//         println!("vec2.len()={:?}", vec2.len());
//         // make sure no errors and we've just read the remaining
//         assert_eq!(vec2.len(), buf.len() - 10);
//     }

//     #[test]
//     fn next_u16() {
//         let buf = test_buf();
//         let mut p = DnsPacket::new(&buf);
//         let len = p.len();
//         // try read 2 bytes when we're 1 byte from the end
//         p.seek(len - 1);
//         assert_eq!(None, p.next_u16());
//         // now try again when we're two bytes away
//         p.seek(len - 2);
//         assert_eq!(true, p.next_u16().is_some());
//     }

//     #[test]
//     fn empty() {
//         let buf = [];
//         let mut p = DnsPacket::new(&buf);
//         assert_eq!(None, p.next());
//         assert_eq!(None, p.next_u8());
//         assert_eq!(None, p.next_u16());
//         assert_eq!(None, p.next_u32());
//         assert_eq!(Vec::<u8>::new(), p.next_bytes(2));
//     }
// }
