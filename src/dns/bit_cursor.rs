///
///A Left-To-Right `BitCursor` for reading and writing bits
///
pub struct BitCursor {
    bits: u16,
    pos: u32,
}

impl Default for BitCursor {
    fn default() -> Self {
        BitCursor { bits: 0, pos: 0 }
    }
}
impl BitCursor {
    #[allow(dead_code)]
    pub fn new_with(bits: u16) -> BitCursor {
        BitCursor {
            bits: bits,
            pos: 0,
        }
    }

    pub fn set(&mut self, bits: u16) {
        self.bits = bits;
        self.pos = 0;
    }

    pub fn next_bool(&mut self) -> bool {
        self.read_and_advance(1) == 1
    }



    pub fn next_u4(&mut self) -> u8 {
        self.read_and_advance(4) as u8
    }

    pub fn write_bool(&mut self, bit: bool) -> bool {
        // println!("mask & bit as u16 {:016b}", self.mask(1) & (bit as u16));
        // self.bits = self.bits | (self.mask(1) & (bit as u16));
        // println!("self.bits {:016b}", self.bits);
        let mut u16_val = bit as u16;
        u16_val = u16_val.rotate_right(1 + self.pos as u32);
        self.bits = self.bits | u16_val;
        self.advance(1)
    }

    // pub fn write(&mut self, bit_cnt: u32, val: u16) -> bool {
    //     return self.write_and_advance(bit_cnt, val);
    // }

    pub fn write_u4(&mut self, val: u8) -> bool {
        self.write_and_advance(4, val as u16)
    }

    #[allow(dead_code)]
    pub fn write_u8(&mut self, val: u8) -> bool {
        self.write_and_advance(8, val as u16)
    }

    #[allow(dead_code)]
    pub fn write_u16(&mut self, val: u16) -> bool {
        self.write_and_advance(16, val)
    }

    fn write_and_advance(&mut self, bit_cnt: u32, val: u16) -> bool {
        let rotated_val = val.rotate_right(bit_cnt + self.pos);
        self.bits = self.bits | rotated_val;
        self.advance(bit_cnt)
    }

    #[allow(dead_code)]
    pub fn next_u8(&mut self) -> u8 {
        self.read_and_advance(8) as u8
    }

    pub fn next_u16(&mut self) -> u16 {
        self.read_and_advance(16)
    }

    ///Returns the next bits by shifting (rotating) left to push the bits to the far right
    ///ans using a mask to get the value.
    fn read_and_advance(&mut self, bits: u32) -> u16 {
        let shifted = self.shift(bits);
        let mask = self.mask(bits);
        let result = shifted & mask;
        trace!("{:016b} - self.bits", self.bits);
        trace!("{:016b} - rotated left", shifted);
        trace!("{:016b} - mask", mask);
        trace!("{:?} - mask", result);
        self.advance(bits);
        result
    }

    // rotate the bits to line up with the mask
    fn shift(&mut self, size: u32) -> u16 {
        let count = self.pos + size;
        self.bits.rotate_left(count)
    }

    fn advance(&mut self, count: u32) -> bool {
        if self.pos + count > 15 {
            return false;
        }
        self.pos += count;
        true
    }

    //
    // Returns a mask to read that many bits. E.g.
    // 0000 0000 0000 0001 to read 1 bit
    // 0000 0000 0000 1111 to read 4 bits
    //
    pub fn mask(&self, bits: u32) -> u16 {
        if bits == 0 {
            return 0;
        }
        let mut mask: usize = 0;
        for i in 0..bits {
            mask = mask + 2usize.pow(i); //pow requires u32
        }
        mask as u16
    }

    pub fn seek(&mut self, pos: u32) -> bool {
        if pos > 16 {
            return false;
        }
        self.pos = pos;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::BitCursor;

    #[test]
    fn mask() {
        let cursor = BitCursor::new_with(12);
        for i in vec![1, 2, 3, 4] {
            let mask = cursor.mask(i);
            println!("{:?} -> {:016b}", i, mask);
            match i {
                1 => assert_eq!(mask, 0b0000_0000_0000_0001),
                2 => assert_eq!(mask, 0b0000_0000_0000_0011),
                3 => assert_eq!(mask, 0b0000_0000_0000_0111),
                4 => assert_eq!(mask, 0b0000_0000_0000_1111),
                _ => unreachable!(),
            }


        }
    }

    #[test]
    fn shift() {
        // read as a bit, then a u4. so true (0001), then 3(0011)
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

    #[test]
    fn write() {
        let mut cursor = BitCursor::new();
        println!("Start {:016b}", cursor.next_u16());
        println!("true as u16 {:016b}", true as u16);
        println!("mask 1 bit {:016b}", cursor.mask(1));
        cursor.write_bool(true);
        cursor.write_bool(true);
        cursor.write_u4(1);
        cursor.write_u8(255);
        cursor.write_u16(1);
        cursor.seek(0);
        println!("next_u16 {:016b}", cursor.next_u16());
        // cursor.write_u4(1);


        // cursor.write_u4();
        // assert_eq!(cursor.next_u16(), 34816); //1000 1000 0000 0000
    }
}
