///Wrapper over a buffer providing seek, next, seek etc.
#[derive(Debug)]
pub struct DnsPacket<'a> {
    buf: &'a [u8],
    pos: usize,
}


impl<'a> DnsPacket<'a> {
    pub fn new(buf: &[u8]) -> DnsPacket {
        return DnsPacket::new_at(buf, 0);
    }

    pub fn new_at(buf: &[u8], pos: usize) -> DnsPacket {
        return DnsPacket {
            buf: buf,
            pos: pos,
        };
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    pub fn seek(&mut self, pos: usize) -> bool {
        if pos > self.len() {
            return false;
        }
        self.pos = pos;
        return true;
    }

    pub fn pos(&self) -> usize {
        return self.pos;
    }

    pub fn len(&self) -> usize {
        return self.buf.len();
    }

    pub fn peek_u8(&self) -> Option<u8> {
        let len = self.buf.len();
        if self.pos >= len {
            return None;
        }
        return Some(self.buf[self.pos]);
    }

    pub fn next_bytes(&mut self, bytes: usize) -> Vec<u8> {
        let mut slice = Vec::with_capacity(bytes);
        for _ in 0..bytes {
            let byte = self.next_u8();
            match byte {
                Some(b) => slice.push(b),
                None => break,
            }
        }
        return slice;
    }

    pub fn next_u8(&mut self) -> Option<u8> {
        match self.peek_u8() {
            Some(byte) => {
                self.pos += 1;
                return Some(byte);
            }
            None => return None,
        }
    }

    ///Return the next u16 IFF there are two bytes to read. If there is only one, None is returned
    ///and pos is not changed
    ///Callers should check and call next_u8 if required
    pub fn next_u16(&mut self) -> Option<u16> {
        let len = self.buf.len();
        if self.pos + 1 >= len {
            return None;
        }
        let byte1 = self.buf[self.pos];
        let byte2 = self.buf[self.pos + 1];
        self.pos += 2;

        return Some(((byte1 as u16) << 8) | byte2 as u16);
    }

    pub fn next_u32(&mut self) -> Option<u32> {
        let len = self.buf.len();
        if (self.pos + 3) >= len {
            return None;
        }

        let val = (self.buf[self.pos] as u32) << 24 | (self.buf[self.pos + 1] as u32) << 16 |
                  (self.buf[self.pos + 2] as u32) << 8 |
                  self.buf[self.pos + 3] as u32;
        self.pos += 4;
        return Some(val);
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
        match self.next_u16() {
            Some(n) => return Some((n, self.pos)),
            None => return None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::DnsPacket;

    fn test_buf() -> Vec<u8> {
        //
        // 00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
        // 00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
        // 00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        //
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111,
                    109, 0, 0, 1, 0, 1];
    }

    #[test]
    fn next_u32() {
        let buf = &test_buf();
        let mut p = DnsPacket::new(buf);

        // println!("u32: {:?}", p.next_u32());
        assert_eq!(141623552, p.next_u32().unwrap());

        let len = p.len();
        // try read 4 bytes when we're 3 byte from the end
        p.seek(len - 3);
        assert_eq!(None, p.next_u32());
        // now try again when we're two bytes away
        p.seek(len - 4);
        assert_eq!(true, p.next_u32().is_some());

    }

    #[test]
    fn seek() {
        let buf = &test_buf();
        let mut p = DnsPacket::new(buf);
        assert_eq!(true, p.seek(5));
        assert_eq!(5, p.pos());
        // can't go past end
        assert_eq!(false, p.seek(1000));
        // position unchanged
        assert_eq!(5, p.pos());
    }

    #[test]
    fn peek_u8() {
        let buf = &test_buf();
        let p = DnsPacket::new(buf);
        assert_eq!(8, p.peek_u8().unwrap());
        // don't move position for a peek
        assert_eq!(0, p.pos());
    }

    #[test]
    fn next_u8() {
        let buf = &test_buf();
        let mut p = DnsPacket::new(buf);
        assert_eq!(8, p.next_u8().unwrap());
        // move position for a peek
        assert_eq!(1, p.pos());
        // return none, don't panic at the end
        let pos = p.len();
        p.seek(pos);
        assert_eq!(None, p.next_u8());
    }

    #[test]
    fn next_u8_boundary() {
        let buf = &test_buf();
        let mut p = DnsPacket::new(buf);
        let len = p.len();
        // go to the end
        assert_eq!(true, p.seek(len));
        // can't read past the end
        assert_eq!(None, p.next_u8());
    }

    #[test]
    fn iterate() {
        let buf = test_buf();
        let packet = DnsPacket::new(&buf);
        for word in packet {
            println!("word: {:016b} {:?}", word.0, word.1);
        }
    }

    #[test]
    fn next_bytes() {
        let buf = test_buf();
        let mut p = DnsPacket::new(&buf);
        // read some bytes
        let vec = p.next_bytes(10);
        assert_eq!(vec.len(), 10);
        // read past the end
        let vec2 = p.next_bytes(100);
        println!("vec2.len()={:?}", vec2.len());
        // make sure no errors and we've just read the remaining
        assert_eq!(vec2.len(), buf.len() - 10);
    }

    #[test]
    fn next_u16() {
        let buf = test_buf();
        let mut p = DnsPacket::new(&buf);
        let len = p.len();
        // try read 2 bytes when we're 1 byte from the end
        p.seek(len - 1);
        assert_eq!(None, p.next_u16());
        // now try again when we're two bytes away
        p.seek(len - 2);
        assert_eq!(true, p.next_u16().is_some());
    }

    #[test]
    fn empty() {
        let buf = [];
        let mut p = DnsPacket::new(&buf);
        assert_eq!(None, p.next());
        assert_eq!(None, p.next_u8());
        assert_eq!(None, p.next_u16());
        assert_eq!(None, p.next_u32());
        assert_eq!(Vec::<u8>::new(), p.next_bytes(2));
    }
}
