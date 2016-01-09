
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

trait BufRead : DirectAccessBuf {

}

trait BufWrite : DirectAccessBuf {
    fn get_buf(&mut self) -> &mut [u8];
    fn write_u8(&mut self, byte: u8) {
        // todo: check
        // todo: return
        self.get_buf()[self.get_pos()] = byte;

        // advance
        let new_pos = self.get_pos() + 1;
        self.set_pos(new_pos);
    }
}

trait DirectAccessBuf {
    fn get_pos(&self) -> usize;
    fn set_pos(&mut self, pos: usize);
    fn reset(&mut self) {
        self.set_pos(0);
    }
}

impl<'a> BufWrite for DnsPacketWriter<'a> {
    fn get_buf(&mut self) -> &mut [u8] {
        return self.buf;
    }
}

impl<'a> DirectAccessBuf for DnsPacketWriter<'a> {
    fn get_pos(&self) -> usize {
        return self.pos;
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
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
