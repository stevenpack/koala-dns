pub trait DirectAccessBuf {
    fn pos(&self) -> usize;
    fn set_pos(&mut self, pos: usize);
    fn len(&self) -> usize;
    fn seek(&mut self, pos: usize) -> bool {
        if pos > self.len() {
            return false;
        }
        self.set_pos(pos);
        return true;
    }
    fn advance(&mut self, count: usize) -> bool {
        let new_pos = self.pos() + count;
        return self.seek(new_pos);
    }
    fn reset(&mut self) {
        self.seek(0);
    }
}

pub trait BufRead : DirectAccessBuf {
    fn buf(&self) -> &Vec<u8>;
    fn capacity(&self) -> usize {
        self.buf().capacity()
    }

    fn peek_u8(&self) -> Option<u8> {
        if self.pos() >= self.len() {
            return None;
        }
        return Some(self.buf()[self.pos()]);
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
    fn buf(&mut self) -> &mut Vec<u8>;

    fn write_u8(&mut self, byte: u8) -> bool {
        println!("u8 self.pos() {:?} >= {:?} self.len()", self.pos(), self.capacity());
        if self.pos() >= self.capacity() {            
            return false;
        }
        let pos = self.pos();
        self.buf().insert(pos, byte);
        self.advance(1);
        return true;
    }

    fn write_u16(&mut self, bytes: u16) -> bool {
        println!("u16 self.pos() {:?} >= {:?} self.len()", self.pos(), self.capacity());
        if self.pos() + 1 >= self.capacity() {
            return false;
        }

        let pos = self.pos();
        // as takes last (rightmost) bits
        //TODO: perf: do this in reverse would prevent shifting
        self.buf().insert(pos, (bytes >> 8) as u8);
        self.buf().insert(pos + 1, bytes as u8);
        self.advance(2);
        return true;
    }

    fn write_u32(&mut self, bytes: u32) -> bool {
        println!("u32 self.pos() {:?} >= {:?} self.len()", self.pos(), self.capacity());
        if self.pos() + 3 >= self.capacity() {
            return false;
        }
        let pos = self.pos();
        self.buf().insert(pos, (bytes >> 24) as u8);
        self.buf().insert(pos + 1, (bytes >> 16) as u8);
        self.buf().insert(pos + 2, (bytes >> 8) as u8);
        self.buf().insert(pos + 3, bytes as u8);
        self.advance(4);
        return true;
    }

    fn write_bytes(&mut self, bytes: Vec<u8>) -> bool {
        println!("u8 self.pos() {:?} >= {:?} self.len()", self.pos(), self.len());
        if self.pos() + bytes.len() > self.capacity() {
            return false;
        }
        for byte in bytes {
            self.write_u8(byte);
        }
        true
    }
}

pub trait IntoBytes {
    fn to_bytes(&self) -> Vec<u8> {
        //TODO: capacity...
        let mut buf = Vec::<u8>::with_capacity(4096);
        self.write(&mut buf);
        debug!("{:?} bytes from to_bytes()", buf.len());
        buf
    }
    fn write(&self, mut buf: &mut Vec<u8>);
}
