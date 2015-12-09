/*
A LeftToRight BitCursor for reading bits
*/
pub struct BitCursor {
    bits: u16,
    pos: usize
}

impl BitCursor {

    pub fn new() -> BitCursor {
        return BitCursor{
            bits: 0,
            pos: 0
        }
    }
 
    #[allow(dead_code)]
    pub fn new_with(bits: u16) -> BitCursor {
        return BitCursor{
            bits: bits,
            pos: 0
        }
    }

    pub fn set(&mut self, bits: u16) {
        self.bits = bits;
        self.pos = 0;
    }

    pub fn next_bool(&mut self) -> bool {
        return self.calc_and_advance(1) == 1;
    }

    pub fn next_u4(&mut self) -> u8 {
        return self.calc_and_advance(4) as u8;
    }

    #[allow(dead_code)]
    pub fn next_u8(&mut self) -> u8 {
        return self.calc_and_advance(8) as u8;
    }

    pub fn next_u16(&mut self) -> u16 {
        return self.calc_and_advance(16);
    }

    fn calc_and_advance(&mut self, bits: usize) -> u16 {
        let shifted = self.shift(bits as u16);
        let mask = self.mask(bits);
        let result = shifted & mask;
        trace!("{:016b} - self.bits", self.bits);
        trace!("{:016b} - rotated left", shifted);
        trace!("{:016b} - mask", mask);
        trace!("{:?} - mask", result);
        self.advance(bits as usize);
        return result;
    }

    //rotate the bits to line up with the mask
    fn shift(&mut self, size: u16) -> u16 {
        let count = (self.pos + size as usize) as u32;
        return self.bits.rotate_left(count)
    }

    fn advance(&mut self, count: usize) {
        self.pos += count;
    }

    /*
    Returns a mask to read that many bits. E.g.
    0000 0000 0000 0001 to read 1 bit
    0000 0000 0000 1111 to read 4 bits
    */
    pub fn mask(&self, bits: usize) -> u16 {
        if bits == 0 {
            return 0;
        }
        let mut mask: usize = 0;
        for i in 0..bits {
            mask = mask + 2usize.pow(i as u32); //pow requires u32
        }
        return mask as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::BitCursor;

    fn test_buf() -> Vec<u8> {
        /*
         00001000 01110001 00000001 00000000 00000000 00000001 00000000 00000000 00000000
         00000000 00000000 00000000 00000101 01111001 01100001 01101000 01101111 01101111
         00000011 01100011 01101111 01101101 00000000 00000000 00000001 00000000 00000001
        */
        return vec![8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];
    }

    #[test]
    fn mask() {
        let cursor = BitCursor::new_with(12);
        for i in vec![1,2,3,4] {
            let mask = cursor.mask(i);
            println!("{:?} -> {:016b}", i, mask);
            match i {
                1 => assert_eq!(mask, 0b0000_0000_0000_0001),
                2 => assert_eq!(mask, 0b0000_0000_0000_0011),
                3 => assert_eq!(mask, 0b0000_0000_0000_0111),
                4 => assert_eq!(mask, 0b0000_0000_0000_1111),
                _ => unreachable!()
            }


        }
    }

    #[test]
    fn shift() {
        //read as a bit, then a u4. so true (1), then 0011
        let mut cursor = BitCursor::new_with(0b1001_1000_0000_0000);
        assert_eq!(0b0011_0000_0000_0001, cursor.shift(1));
        cursor.advance(1);
        assert_eq!(0b0000_0000_0001_0011, cursor.shift(4));
    }

    #[test]
    fn next() {
        let mut cursor = BitCursor::new_with(0b0000_1000_0111_0001); //1st word
        assert_eq!(2161, cursor.next_u16());   //id
        cursor.set(0b0000_0001_0000_0000); //2nd word
        assert_eq!(false, cursor.next_bool()); //qr
        assert_eq!(0, cursor.next_u4());       //opcode
        assert_eq!(false, cursor.next_bool()); //aa
        assert_eq!(false, cursor.next_bool()); //tc
        assert_eq!(true, cursor.next_bool()); //rd
        assert_eq!(false, cursor.next_bool()); //ra
        assert_eq!(0, cursor.next_u4());       //z
        assert_eq!(0, cursor.next_u4());       //rcode
    }
}
