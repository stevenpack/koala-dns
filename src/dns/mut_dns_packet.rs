use buf::*;

#[derive(Debug)]
pub struct MutDnsPacket<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> MutDnsPacket<'a> {
    pub fn new(buf: &mut [u8]) -> MutDnsPacket {
        return MutDnsPacket::new_at(buf, 0);
    }

    pub fn new_at(buf: &mut [u8], pos: usize) -> MutDnsPacket {
        return MutDnsPacket {
            buf: buf,
            pos: pos,
        };
    }
}


impl<'a> BufWrite for MutDnsPacket<'a> {
    fn buf(&mut self) -> &mut [u8] {
        return self.buf;
    }
}

impl<'a> BufRead for MutDnsPacket<'a> {
    fn buf(&self) -> &[u8] {
        return self.buf;
    }
}

impl<'a> DirectAccessBuf for MutDnsPacket<'a> {
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

    use super::MutDnsPacket;
    use buf::BufWrite;

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
        let mut packet = MutDnsPacket::new(buf);
        packet.write_u8(7);
        packet.write_u8(7);
        packet.write_u8(7);
        // packet.seek(0);
        // assert_eq!(7, packet.peek_u8().unwrap());
        println!("{:?}", packet);
    }
}
