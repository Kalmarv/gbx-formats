//! Blowfish (Schneier, 1993) as Nadeo's packs use it: the standard cipher with its key's bytes taken four at a time as
//! little-endian words, and each 8-byte block read as two little-endian halves, the first the left one.
#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "a cipher over fixed tables: every index is a byte into a 256-entry box or a constant into the P-array,               and a block's halves are its two 32-bit words"
)]

use super::pi::{P, S};

/// A Blowfish key schedule.
#[derive(Clone)]
pub(crate) struct Blowfish {
    p: [u32; 18],
    /// The four S-boxes, one after another.
    s: Box<[u32; 1024]>,
}

impl std::fmt::Debug for Blowfish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Blowfish { .. }")
    }
}

impl Blowfish {
    /// The schedule of a key of 1 to 56 bytes (a pack's key is 16); its bytes are cycled through the P-array.
    pub(crate) fn new(key: &[u8]) -> Blowfish {
        let mut bf = Blowfish { p: P, s: Box::new(S) };
        let mut key_bytes = key.iter().copied().cycle();
        for p in &mut bf.p {
            let mut word = 0u32;
            for shift in [0, 8, 16, 24] {
                word |= u32::from(key_bytes.next().unwrap_or(0)) << shift;
            }
            *p ^= word;
        }
        let (mut l, mut r) = (0, 0);
        for i in (0..18).step_by(2) {
            (l, r) = bf.encrypt(l, r);
            bf.p[i] = l;
            bf.p[i + 1] = r;
        }
        for i in (0..1024).step_by(2) {
            (l, r) = bf.encrypt(l, r);
            bf.s[i] = l;
            bf.s[i + 1] = r;
        }
        bf
    }

    fn f(&self, x: u32) -> u32 {
        // each index is a byte of `x` plus a multiple of 256 below 1024
        let s = &self.s;
        let [a, b, c, d] = x.to_be_bytes().map(usize::from);
        (s[a].wrapping_add(s[256 + b]) ^ s[512 + c]).wrapping_add(s[768 + d])
    }

    fn encrypt(&self, mut l: u32, mut r: u32) -> (u32, u32) {
        for &p in &self.p[..16] {
            l ^= p;
            r ^= self.f(l);
            (l, r) = (r, l);
        }
        (r ^ self.p[17], l ^ self.p[16])
    }

    fn decrypt(&self, mut l: u32, mut r: u32) -> (u32, u32) {
        for &p in self.p[2..].iter().rev() {
            l ^= p;
            r ^= self.f(l);
            (l, r) = (r, l);
        }
        (r ^ self.p[0], l ^ self.p[1])
    }

    /// Decrypts one block, read as a little-endian `u64` (its left half the low word).
    pub(crate) fn decrypt_block(&self, block: u64) -> u64 {
        let (l, r) = self.decrypt(block as u32, (block >> 32) as u32);
        u64::from(l) | (u64::from(r) << 32)
    }

    /// Encrypts one block, read as [`Blowfish::decrypt_block`] reads it.
    #[cfg(test)]
    pub(crate) fn encrypt_block(&self, block: u64) -> u64 {
        let (l, r) = self.encrypt(block as u32, (block >> 32) as u32);
        u64::from(l) | (u64::from(r) << 32)
    }
}

#[cfg(test)]
mod tests {
    use super::Blowfish;

    /// Schneier's published test vectors (key, plaintext, ciphertext, each big-endian), read through the little-endian
    /// convention by reversing every 4-byte word: the same cipher.
    #[test]
    fn schneier_vectors() {
        let words = |x: u64| -> u64 {
            let b = x.to_be_bytes();
            let l = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
            let r = u32::from_be_bytes([b[4], b[5], b[6], b[7]]);
            u64::from(l) | (u64::from(r) << 32)
        };
        let key_le = |k: u64| -> Vec<u8> {
            let b = k.to_be_bytes();
            vec![b[3], b[2], b[1], b[0], b[7], b[6], b[5], b[4]]
        };
        for (key, plain, cipher) in [
            (
                0x0000_0000_0000_0000u64,
                0x0000_0000_0000_0000u64,
                0x4EF9_9745_6198_DD78u64,
            ),
            (
                0xFFFF_FFFF_FFFF_FFFF,
                0xFFFF_FFFF_FFFF_FFFF,
                0x5186_6FD5_B85E_CB8A,
            ),
            (
                0x3000_0000_0000_0000,
                0x1000_0000_0000_0001,
                0x7D85_6F9A_6130_63F2,
            ),
            (
                0x0123_4567_89AB_CDEF,
                0x1111_1111_1111_1111,
                0x61F9_C380_2281_B096,
            ),
            (
                0xFEDC_BA98_7654_3210,
                0x0123_4567_89AB_CDEF,
                0x0ACE_AB0F_C6A0_A28D,
            ),
        ] {
            let bf = Blowfish::new(&key_le(key));
            assert_eq!(bf.encrypt_block(words(plain)), words(cipher), "key {key:016X}");
            assert_eq!(bf.decrypt_block(words(cipher)), words(plain), "key {key:016X}");
        }
    }
}
