use std::char;
#[derive(Debug)]
pub struct DnsPacket<'a> {
    buf: &'a [u8],
    pos: usize
}

impl<'a> DnsPacket<'a> {
    pub fn new(buf: &[u8]) -> DnsPacket {
        return DnsPacket::new_at(buf, 0);
    }

    pub fn new_at(buf: &[u8], pos: usize) -> DnsPacket {
        //debug!("{:?}", buf);
        return DnsPacket {
            buf: buf,
            pos: pos
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    pub fn seek(&mut self, pos: usize) {
        //todo: safety
        self.pos = pos;
    }

    #[allow(dead_code)]
    pub fn pos(&self) -> usize {
        return self.pos;
    }

    #[allow(dead_code)]
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
                None => break
            }
        }
        return slice;
    }

    pub fn next_u8(&mut self) -> Option<u8> {
        match self.peek_u8() {
            Some(byte) => {
                self.pos += 1;
                return Some(byte);
            },
            None => return None
        }
    }

    pub fn next_u16(&mut self) -> Option<u16> {
        //todo: check if this is necessary... allowing to only read 8 of the 16 bits
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

    pub fn next_u32(&mut self) -> Option<u32> {
        let len = self.buf.len();
        if (self.pos + 4) >= len {
            return None;
        }

        let val = (self.buf[self.pos] as u32) << 24 |
                  (self.buf[self.pos + 1] as u32) << 16 |
                  (self.buf[self.pos + 2] as u32) << 8 |
                   self.buf[self.pos + 3] as u32;
        self.pos += 4;
        return Some(val);
    }

    #[allow(dead_code)]
    pub fn dump2(&mut self) {
        let current_pos = self.pos();
        let mut marker;
        self.reset();
        loop {
            match self.next_u8() {
                Some(word) => {
                    marker = format!("< byte: {}", self.pos - 1);
                    if self.pos - 1 == current_pos {
                        marker = format!("{} self.pos={}", marker, self.pos);
                    }
                    println!("{:08b} {} {:?}", word, marker, char::from_u32(word as u32));
                },
                None => break
            }
        }
        self.pos = current_pos;
    }

    pub fn dump(&mut self) {
        let current_pos = self.pos();
        let mut marker;
        self.reset();
        loop {
            match self.next_u16() {
                Some(word) => {
                    marker = format!("< byte: {}-{}", self.pos - 2, self.pos - 1);
                    if self.pos - 2 == current_pos || self.pos - 1 == current_pos {
                        marker = format!("{} self.pos={}", marker, self.pos);
                    }
                    println!("{:016b} {}", word, marker);
                },
                None => break
            }
        }
        self.pos = current_pos;
    }
}

//or impl Iterator for WordIterator, impl Iterator for OctetIterator
//todo: new (english) word. Hextet, for 16 bytes.
impl<'a> Iterator for DnsPacket<'a> {
    type Item = (u16, usize);

    /*
    Returns two octets in the order they expressed in the spec. I.e. first byte shifted to the left
    */
    fn next(&mut self) -> Option<(u16, usize)> {
        match self.next_u16() {
            Some(n) => return Some((n, self.pos)),
            None => return None
        }
    }
 }
#[cfg(test)]
mod tests {
    use super::DnsPacket;

    fn test_buf() -> Vec<u8> {
        /*
         00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
         00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
         00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        */
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    #[test]
    #[ignore]
    fn next_u32() {
        println!("u32: {:?}", DnsPacket::new(&test_buf()).next_u32());
    }

    #[test]
    #[ignore]
    fn iterate() {
        let buf = test_buf();
        let packet = DnsPacket::new(&buf);
        for word in packet {
            //println!("word: {:016b}", word);
        }
    }
}
