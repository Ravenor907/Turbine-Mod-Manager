//! DEFLATE decompression (RFC 1951), enough for zip archives. Written here so the engine has no outside code in the
//! path every downloaded file goes through. Output is capped so a hostile archive can't fill the disk or memory.

#[derive(Debug, PartialEq)]
pub enum InflateError {
    Truncated,
    BadBlock,
    BadCode,
    BadDistance,
    TooLarge,
}

impl std::fmt::Display for InflateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InflateError::Truncated => "the compressed data ends early",
            InflateError::BadBlock => "the compressed data is damaged (bad block)",
            InflateError::BadCode => "the compressed data is damaged (bad code)",
            InflateError::BadDistance => "the compressed data is damaged (bad distance)",
            InflateError::TooLarge => "the file unpacks to more than its stated size",
        })
    }
}

impl std::error::Error for InflateError {}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
    nbits: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Bits { data, pos: 0, bit: 0, nbits: 0 }
    }

    fn need(&mut self, n: u32) -> Result<(), InflateError> {
        while self.nbits < n {
            let b = *self.data.get(self.pos).ok_or(InflateError::Truncated)?;
            self.pos += 1;
            self.bit |= (b as u32) << self.nbits;
            self.nbits += 8;
        }
        Ok(())
    }

    fn take(&mut self, n: u32) -> Result<u32, InflateError> {
        if n == 0 {
            return Ok(0);
        }
        self.need(n)?;
        let v = self.bit & ((1u32 << n) - 1);
        self.bit >>= n;
        self.nbits -= n;
        Ok(v)
    }

    fn align(&mut self) {
        self.bit = 0;
        self.nbits = 0;
    }
}

/// Canonical Huffman decoding table: counts per length and symbols in order.
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Self, InflateError> {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        // Over-subscribed sets are invalid; incomplete ones are allowed (a single code is legal).
        let mut left: i32 = 1;
        for &c in counts.iter().skip(1) {
            left <<= 1;
            left -= c as i32;
            if left < 0 {
                return Err(InflateError::BadCode);
            }
        }
        let mut offs = [0u16; 16];
        for i in 1..15 {
            offs[i + 1] = offs[i] + counts[i];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offs[l as usize] as usize] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huffman { counts, symbols })
    }

    fn decode(&self, bits: &mut Bits) -> Result<u16, InflateError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= bits.take(1)? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(InflateError::BadCode)
    }
}

const LEN_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289,
    16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// Decompresses raw DEFLATE data. `limit` is the most bytes the output may reach.
pub fn inflate(data: &[u8], limit: usize) -> Result<Vec<u8>, InflateError> {
    let mut out: Vec<u8> = Vec::with_capacity(limit.min(1 << 24));
    let mut bits = Bits::new(data);
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => {
                bits.align();
                let p = bits.pos;
                let hdr = data.get(p..p + 4).ok_or(InflateError::Truncated)?;
                let len = u16::from_le_bytes([hdr[0], hdr[1]]) as usize;
                let nlen = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
                if len != (!nlen & 0xFFFF) {
                    return Err(InflateError::BadBlock);
                }
                let chunk = data.get(p + 4..p + 4 + len).ok_or(InflateError::Truncated)?;
                if out.len() + len > limit {
                    return Err(InflateError::TooLarge);
                }
                out.extend_from_slice(chunk);
                bits.pos = p + 4 + len;
            }
            1 => {
                let mut l = [0u8; 288];
                l[..144].fill(8);
                l[144..256].fill(9);
                l[256..280].fill(7);
                l[280..].fill(8);
                let lit = Huffman::new(&l)?;
                let dist = Huffman::new(&[5u8; 30])?;
                block(&mut bits, &mut out, &lit, &dist, limit)?;
            }
            2 => {
                let hlit = bits.take(5)? as usize + 257;
                let hdist = bits.take(5)? as usize + 1;
                let hclen = bits.take(4)? as usize + 4;
                const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut cl = [0u8; 19];
                for &i in ORDER.iter().take(hclen) {
                    cl[i] = bits.take(3)? as u8;
                }
                let clh = Huffman::new(&cl)?;
                let mut lengths = vec![0u8; hlit + hdist];
                let mut i = 0;
                while i < hlit + hdist {
                    let sym = clh.decode(&mut bits)?;
                    let (val, rep) = match sym {
                        0..=15 => (sym as u8, 1),
                        16 => {
                            let prev = *lengths.get(i.wrapping_sub(1)).ok_or(InflateError::BadBlock)?;
                            (prev, 3 + bits.take(2)? as usize)
                        }
                        17 => (0, 3 + bits.take(3)? as usize),
                        18 => (0, 11 + bits.take(7)? as usize),
                        _ => return Err(InflateError::BadCode),
                    };
                    if i + rep > hlit + hdist {
                        return Err(InflateError::BadBlock);
                    }
                    lengths[i..i + rep].fill(val);
                    i += rep;
                }
                if lengths[256] == 0 {
                    return Err(InflateError::BadBlock);
                }
                let lit = Huffman::new(&lengths[..hlit])?;
                let dist = Huffman::new(&lengths[hlit..])?;
                block(&mut bits, &mut out, &lit, &dist, limit)?;
            }
            _ => return Err(InflateError::BadBlock),
        }
        if last {
            return Ok(out);
        }
    }
}

fn block(bits: &mut Bits, out: &mut Vec<u8>, lit: &Huffman, dist: &Huffman, limit: usize) -> Result<(), InflateError> {
    loop {
        let sym = lit.decode(bits)? as usize;
        if sym < 256 {
            if out.len() >= limit {
                return Err(InflateError::TooLarge);
            }
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let i = sym - 257;
            if i >= 29 {
                return Err(InflateError::BadCode);
            }
            let len = LEN_BASE[i] as usize + bits.take(LEN_EXTRA[i] as u32)? as usize;
            let d = dist.decode(bits)? as usize;
            if d >= 30 {
                return Err(InflateError::BadDistance);
            }
            let back = DIST_BASE[d] as usize + bits.take(DIST_EXTRA[d] as u32)? as usize;
            if back > out.len() {
                return Err(InflateError::BadDistance);
            }
            if out.len() + len > limit {
                return Err(InflateError::TooLarge);
            }
            let start = out.len() - back;
            for k in 0..len {
                let b = out[start + k];
                out.push(b);
            }
        }
    }
}
