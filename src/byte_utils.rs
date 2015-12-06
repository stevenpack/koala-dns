#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::fmt::{Binary, Formatter, Error};

pub fn read_bit(buf: &[u8], mask: u8) -> bool {
    read_bit_at(buf, mask, 0)
}

pub fn read_bit_at(buf: &[u8], mask: u8, byte_offset: usize) -> bool {
    //println!("byte {:08b}", buf[byte_offset]);
    //println!("mask {:08b}", mask);
    let result = buf[byte_offset] & mask;
    //println!("res: {:08b}", result);
    return result == mask;
}

pub fn read_u4(buf: &[u8]) -> u8 {
    return read_u4_at(buf, 0, 0)
}
pub fn read_u4_at(buf: &[u8], byte_offset: usize, bit_offset: u8) -> u8 {
    return (buf[byte_offset] >> bit_offset) & 0b0000_1111;
}

pub fn read_u16(buf: &[u8]) -> u16 {
    return read_u16_at(buf, 0);
}

pub fn read_u16_at(buf: &[u8], byte_offset: usize) -> u16 {
    let byte1: u16 = buf[byte_offset] as u16;
    let byte2: u16 = buf[byte_offset + 1] as u16;
    return byte1 << 8 | byte2;
}

pub fn read_u32(buf: &[u8]) -> u32 {
    let byte1: u32 = buf[0] as u32;
    let byte2: u32 = buf[1] as u32;
    let byte3: u32 = buf[2] as u32;
    let byte4: u32 = buf[3] as u32;
    return byte1 << 24 |
           byte2 << 16 |
           byte3 << 8 |
           byte4;
}

fn format(buf: &[u8]) -> String {
    let mut fmt_str = String::with_capacity(buf.len() * 8);
    for byte in buf.iter() {
        fmt_str.push_str(&format!("{:08b} ", byte));
    }
    return fmt_str;
    //return String::new();
}

#[cfg(test)]
mod tests {
    //                               1  1  1  1  1  1
    // 0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
    // +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    // |                      ID                       |
    // +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    // |QR|   Opcode  |AA|TC|RD|RA|   Z    |   RCODE   |
    // +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    //test data is a buffer encoding the following dns header in the above format
    // ;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 2161
    // ;; flags: qr rd ra; QUERY: 1, ANSWER: 3, AUTHORITY: 0, ADDITIONAL: 0
    const BUF: [u8; 27] = [8, 113, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 5, 121, 97, 104, 111, 111, 3, 99, 111, 109, 0, 0, 1, 0, 1];

    #[test]
    fn _setup() {
        //assert_eq!(super::read_u4_at(&BUF, 17), 0);
        println!("Buffer for most tests is:");
        println!("{}", super::format(&BUF));
    }

    #[test]
    fn read_u16() {
        //8 :   0000 1000
        //113:  0111 0001
        //2161: 0000 1000 0111 0001
        let val = super::read_u16(&BUF);
        assert_eq!(val, 2161);
    }

    #[test]
    fn read_u32() {
        //8 :   0000 1000
        //113:  0111 0001
        //1:    0000 0001
        //0:    0000 0000
        //141623552: 0000 1000 0111 0001 0000 0001 0000 0000
        let val = super::read_u32(&BUF);
        assert_eq!(val, 141623552);
    }

    #[test]
    fn read_bit() {
        let buf: [u8; 1] = [1];
        assert_eq!(super::read_bit(&buf, 0b0000_0001), true);
        assert_eq!(super::read_bit(&buf, 0b0000_0010), false);
    }

    #[test]
    fn read_bit_at() {
        //First 16 bits of buffer looks like:
        //00001000 01110001
        //try read each bit

        //byte 1
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0001, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0010, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0100, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b0000_1000, 0), true);  //1
        assert_eq!(super::read_bit_at(&BUF, 0b0001_0000, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b0010_0000, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b0100_0000, 0), false); //0
        assert_eq!(super::read_bit_at(&BUF, 0b1000_0000, 0), false); //0

        //byte 2
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0001, 1), true);
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0010, 1), false);
        assert_eq!(super::read_bit_at(&BUF, 0b0000_0100, 1), false);
        assert_eq!(super::read_bit_at(&BUF, 0b0000_1000, 1), false);
        assert_eq!(super::read_bit_at(&BUF, 0b0001_0000, 1), true);
        assert_eq!(super::read_bit_at(&BUF, 0b0010_0000, 1), true);
        assert_eq!(super::read_bit_at(&BUF, 0b0100_0000, 1), true);
        assert_eq!(super::read_bit_at(&BUF, 0b1000_0000, 1), false);
    }

    #[test]
    fn read_u4() {
        //255 (1111 1111) as a u4 is 15 (0000 1111)
        assert_eq!(super::read_u4(&[255]), 15);
    }

    #[test]
    fn read_u4_at() {
        //First 24 bits of buffer looks like:
        //00001000 01110001 00000001
        //try read a u4 from the 17th bit
        assert_eq!(super::read_u4_at(&BUF, 2, 0), 1);
        assert_eq!(super::read_u4_at(&BUF, 2, 1), 0);
    }

 }
