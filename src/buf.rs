
#[derive(Debug)]
pub struct DnsPacketWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> DnsPacketWriter<'a> {
    pub fn new(buf: &mut [u8]) -> DnsPacketWriter {
        return DnsPacketWriter::new_at(buf, 0);
    }

    pub fn new_at(buf: &mut [u8], pos: usize) -> DnsPacketWriter {
        return DnsPacketWriter {
            buf: buf,
            pos: pos,
        };
    }
}

pub trait BufRead : DirectAccessBuf {
    fn buf(&self) -> &[u8];

    fn peek_u8(&self) -> Option<u8> {
        let len = self.buf().len();
        if self.pos() >= self.len() {
            return None;
        }
        return Some(self.buf()[self.pos()]);
    }

    fn len(&self) -> usize {
        return self.buf().len();
    }


    fn next_bytes(&mut self, bytes: usize) -> Vec<u8> {
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

    fn next_u8(&mut self) -> Option<u8> {
        match self.peek_u8() {
            Some(byte) => {
                self.advance(1);
                return Some(byte);
            }
            None => return None,
        }
    }

    ///Return the next u16 IFF there are two bytes to read. If there is only one, None is returned
    ///and pos is not changed
    ///Callers should check and call next_u8 if required
    fn next_u16(&mut self) -> Option<u16> {
        let len = self.len();
        if self.pos() + 1 >= len {
            return None;
        }
        let byte1 = self.buf()[self.pos()];
        let byte2 = self.buf()[self.pos() + 1];
        self.advance(2);

        return Some(((byte1 as u16) << 8) | byte2 as u16);
    }

    fn next_u32(&mut self) -> Option<u32> {
        let len = self.len();
        if (self.pos() + 3) >= len {
            return None;
        }

        let val = (self.buf()[self.pos()] as u32) << 24 |
                  (self.buf()[self.pos() + 1] as u32) << 16 |
                  (self.buf()[self.pos() + 2] as u32) << 8 |
                  self.buf()[self.pos() + 3] as u32;
        self.advance(4);
        return Some(val);
    }
}

pub trait BufWrite : BufRead {
    fn buf(&mut self) -> &mut [u8];
    fn write_u8(&mut self, byte: u8) {
        // todo: check
        // todo: return
        self.buf()[self.pos()] = byte;
        self.advance(1);
    }
}

pub trait DirectAccessBuf {
    fn pos(&self) -> usize;
    fn seek(&mut self, pos: usize) -> bool;
    fn advance(&mut self, count: usize) -> bool {
        let new_pos = self.pos() + count;
        return self.seek(new_pos);
    }
    fn reset(&mut self) {
        self.seek(0);
    }
}

impl<'a> BufWrite for DnsPacketWriter<'a> {
    fn buf(&mut self) -> &mut [u8] {
        return self.buf;
    }
}

impl<'a> BufRead for DnsPacketWriter<'a> {
    fn buf(&self) -> &[u8] {
        return self.buf;
    }
}

impl<'a> DirectAccessBuf for DnsPacketWriter<'a> {
    fn pos(&self) -> usize {
        return self.pos;
    }
    fn seek(&mut self, pos: usize) -> bool {
        self.pos = pos;
        // todo: check
        return true;
    }
}


mod tests {

    use super::DnsPacketWriter;
    use super::BufWrite;

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
    fn write_u8() {
        let mut vec = test_buf();
        let mut buf = vec.as_mut_slice();
        let mut packet = DnsPacketWriter::new(buf);
        packet.write_u8(7);
        packet.write_u8(7);
        packet.write_u8(7);
        // packet.seek(0);
        // assert_eq!(7, packet.peek_u8().unwrap());
        println!("{:?}", packet);
    }
}
