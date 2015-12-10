#[derive(Debug)]
pub struct DnsPacket<'a> {
    buf: &'a [u8],
    pub pos: usize
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

    pub fn next_u16(&mut self) -> Option<u16> {
        let len = self.buf.len();
        if self.pos >= len {
            self.pos = 0;
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
    fn iterate() {
        let buf = test_buf();
        let packet = DnsPacket::new(&buf);
        for word in packet {
            //println!("word: {:016b}", word);
        }
    }
}
