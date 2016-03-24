use buf::*;

#[derive(Debug)]
pub struct MutDnsPacket<'a> {
    buf: &'a mut Vec<u8>,
    pos: usize,
}

impl<'a> MutDnsPacket<'a> {
    pub fn new(buf: &mut Vec<u8>) -> MutDnsPacket {
        return MutDnsPacket::new_at(buf, 0);
    }

    pub fn new_at(buf: &mut Vec<u8>, pos: usize) -> MutDnsPacket {
        return MutDnsPacket {
            buf: buf,
            pos: pos,
        };
    }
}

impl<'a> BufWrite for MutDnsPacket<'a> {
    fn buf(&mut self) -> &mut Vec<u8> {
        return self.buf;
    }
}

impl<'a> BufRead for MutDnsPacket<'a> {
    fn buf(&self) -> &Vec<u8> {
        return self.buf;
    }
}

impl<'a> DirectAccessBuf for MutDnsPacket<'a> {
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
//TODO: this is a copy and paste from DnsPacket
///Iterate each 16bit word in the packet
impl<'a> Iterator for MutDnsPacket<'a> {
    ///2 octets of data and the position
    type Item = (u16, usize);

    ///
    ///Returns two octets in the order they expressed in the spec. I.e. first byte shifted to the left
    ///
    fn next(&mut self) -> Option<Self::Item> {
        return self.next_u16().and_then(|n| return Some((n, self.pos)));
    }
}

// #[cfg(test)]
// mod tests {

//     use super::MutDnsPacket;    
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
//     fn write_u8() {
//         let mut buf = test_buf();
//         //let mut buf = vec.as_mut_slice();
//         let mut packet = MutDnsPacket::new(&mut buf);
//         packet.write_u8(7);
//         packet.write_u8(8);
//         packet.write_u8(9);
//         packet.seek(0);
//         assert_eq!(7, packet.next_u8().unwrap());
//         assert_eq!(8, packet.next_u8().unwrap());
//         assert_eq!(9, packet.next_u8().unwrap());
//     }

    // #[test]
    // fn write_u16() {
    //     let mut vec = test_buf();
    //     let mut buf = vec.as_mut_slice();
    //     let mut packet = MutDnsPacket::new(buf);
    //     packet.write_u16(2161);
    //     packet.write_u16(1);
    //     packet.seek(0);
    //     println!("{:?}", packet);
    //     assert_eq!(2161, packet.next_u16().unwrap());
    //     assert_eq!(1, packet.next_u16().unwrap());
    // }

    // #[test]
    // fn write_u16_bounds() {
    //     let mut vec = vec![0, 0, 0, 0];
    //     let mut buf = vec.as_mut_slice();
    //     let mut packet = MutDnsPacket::new(buf);
    //     assert_eq!(true, packet.write_u16(1));
    //     assert_eq!(true, packet.write_u16(1));
    //     assert_eq!(false, packet.write_u16(1));
    //     println!("{:?}", packet);
    // }

    // #[test]
    // fn write_u32() {
    //     let mut vec = vec![0, 0, 0, 0];
    //     let mut buf = vec.as_mut_slice();
    //     let mut packet = MutDnsPacket::new(buf);
    //     assert_eq!(true, packet.write_u32(123456789));
    //     println!("{:?}", packet);
    //     packet.seek(0);
    //     assert_eq!(123456789, packet.next_u32().unwrap());
    // }
//}
