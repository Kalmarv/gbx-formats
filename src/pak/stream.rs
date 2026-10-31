//! A pack file's plaintext, produced as its reader reaches it.
//!
//! The ciphertext is Blowfish in CBC mode (each block's plaintext is its decryption XORed with the block before it,
//! the first with the 8-byte IV), read 256 bytes at a time; at the start of each 256-byte chunk the chain value takes
//! in, by XOR, what the dummy writes made since the last chunk accumulated ([`FileStream::dummy_write`]). A compressed
//! file's plaintext is the zlib stream that ciphertext decrypts to, inflated 1024 bytes at a time: a block is
//! inflated when the reader first needs a byte of it, and takes in a new chunk of ciphertext whenever the inflater has
//! used up the last one. So which writes reach a chunk depends on how far the reader had read when the chunk was
//! needed, and a file decrypts only when it is read in the order and to the extent the game reads it.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::blowfish::Blowfish;
use crate::error::{Error, Result, Space, bad};
use primase_foundations::compress::Inflater;
use std::sync::Arc;

/// How much ciphertext the cipher reads at a time, and how often the chain takes in the dummy writes.
const CHUNK: usize = 256;
/// How much plaintext the inflater produces at a time.
const BLOCK: usize = 1024;

/// A `u64`'s low and high 32 bits.
fn halves(x: u64) -> (u32, u32) {
    let [a, b, c, d, e, f, g, h] = x.to_le_bytes();
    (u32::from_le_bytes([a, b, c, d]), u32::from_le_bytes([e, f, g, h]))
}

/// The cipher over a file's data: what follows its IV, read from the pack's bytes.
#[derive(Debug)]
struct Cipher {
    /// `None` for a file stored in the clear.
    blowfish: Option<Blowfish>,
    pack: Arc<[u8]>,
    /// The pack offset of the next byte of ciphertext to read.
    next: usize,
    /// Where the ciphertext the file may use ends, in the pack.
    end: usize,
    iv: u64,
    /// What the dummy writes since the last chunk accumulated.
    iv_xor: u64,
}

impl Cipher {
    /// Folds 4 bytes into the accumulator, as the game's "dummy write" does.
    fn dummy_write(&mut self, data: [u8; 4]) {
        for b in data {
            let (lo, hi) = halves(self.iv_xor);
            let lo = (u32::from(b) | 0xAA) ^ ((lo << 13) | (hi >> 19));
            let (_, hi) = halves(self.iv_xor << 13);
            self.iv_xor = (u64::from(hi) << 32) | u64::from(lo);
        }
    }

    /// Appends the next chunk of plaintext (up to [`CHUNK`] bytes, whole blocks) to `out`, folding the accumulated
    /// writes into the chain first; returns how many bytes it appended, 0 at the end of the ciphertext.
    fn chunk(&mut self, out: &mut Vec<u8>) -> usize {
        let left = self.end.saturating_sub(self.next);
        // whole blocks only
        let n = left.min(CHUNK) & !7;
        let Some(bytes) = self.pack.get(self.next..self.next.saturating_add(n)) else {
            return 0;
        };
        self.iv ^= self.iv_xor;
        self.iv_xor = 0;
        match &self.blowfish {
            None => out.extend_from_slice(bytes),
            Some(bf) => {
                for &block in bytes.as_chunks::<8>().0 {
                    let c = u64::from_le_bytes(block);
                    out.extend_from_slice(&(bf.decrypt_block(c) ^ self.iv).to_le_bytes());
                    self.iv = c;
                }
            }
        }
        self.next = self.next.saturating_add(n);
        n
    }
}

/// An inflater's refusal, while producing the plaintext at `at` of the file whose data is at `data_offset` in the pack.
fn inflate_error(data_offset: usize, at: usize, e: &primase_foundations::compress::Error) -> Error {
    bad(
        Space::File,
        at,
        format!("inflating the pack file (data at pack offset {data_offset}): {e}"),
    )
}

/// The inflater of a compressed file, and the chunk of decrypted input it is using.
#[derive(Debug)]
struct Inflate {
    inflater: Inflater,
    input: Vec<u8>,
    used: usize,
    ended: bool,
}

/// A pack file's plaintext, decrypted (and inflated) as far as its reader has asked: see the module documentation.
/// [`crate::pak::Pak::open`] makes one.
#[derive(Debug)]
pub struct FileStream {
    cipher: Cipher,
    inflate: Option<Inflate>,
    /// The plaintext so far (an uncompressed file's may run past `size`, to the end of its last chunk).
    out: Vec<u8>,
    size: usize,
    /// The pack offset of the file's data, for errors.
    data_offset: usize,
}

impl FileStream {
    pub(super) fn new(
        blowfish: Option<Blowfish>,
        pack: Arc<[u8]>,
        start: usize,
        end: usize,
        iv: u64,
        size: usize,
        compressed: bool,
    ) -> Result<FileStream> {
        let inflate = if compressed {
            Some(Inflate {
                inflater: Inflater::zlib().map_err(|e| bad(Space::Pack, start, e.to_string()))?,
                input: Vec::with_capacity(CHUNK),
                used: 0,
                ended: false,
            })
        } else {
            None
        };
        Ok(FileStream {
            cipher: Cipher {
                blowfish,
                pack,
                next: start,
                end,
                iv,
                iv_xor: 0,
            },
            inflate,
            out: Vec::new(),
            size,
            data_offset: start,
        })
    }

    /// The file's size: how many bytes of plaintext it has.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The dummy write: folds `data` into the cipher's chain at the start of the next chunk it reads.
    pub fn dummy_write(&mut self, data: [u8; 4]) {
        self.cipher.dummy_write(data);
    }

    /// The plaintext bytes `start..end`, decrypting (and inflating) whatever of them has not been yet. `end` past the
    /// file's size is refused.
    pub fn read(&mut self, start: usize, end: usize) -> Result<&[u8]> {
        if end > self.size {
            return Err(bad(
                Space::File,
                start,
                format!("bytes up to {end} of a {}-byte pack file", self.size),
            ));
        }
        while self.out.len() < end {
            self.next_piece()?;
        }
        self.out
            .get(start..end)
            .ok_or_else(|| bad(Space::File, start, format!("bytes {start}..{end}")))
    }

    /// Produces the next chunk (uncompressed) or block (compressed) of plaintext.
    fn next_piece(&mut self) -> Result<()> {
        let at = self.out.len();
        let Some(z) = &mut self.inflate else {
            if self.cipher.chunk(&mut self.out) == 0 {
                return Err(bad(
                    Space::File,
                    at,
                    format!(
                        "the pack's ciphertext ends {at} bytes into a {}-byte file",
                        self.size
                    ),
                ));
            }
            return Ok(());
        };
        let mut block = [0u8; BLOCK];
        let mut filled = 0;
        while filled < BLOCK && !z.ended {
            if z.used >= z.input.len() {
                z.input.clear();
                z.used = 0;
                if self.cipher.chunk(&mut z.input) == 0 {
                    break;
                }
            }
            let input = z.input.get(z.used..).unwrap_or_default();
            let room = block.get_mut(filled..).unwrap_or_default();
            let data_offset = self.data_offset;
            let step = z
                .inflater
                .inflate(input, room)
                .map_err(|e| inflate_error(data_offset, at, &e))?;
            z.used = z.used.saturating_add(step.consumed);
            filled = filled.saturating_add(step.produced);
            z.ended = step.ended;
            if step.consumed == 0 && step.produced == 0 && !step.ended {
                return Err(bad(Space::File, at, "the zlib stream makes no progress"));
            }
        }
        if filled == 0 {
            return Err(bad(
                Space::File,
                at,
                format!("the zlib stream ends {at} bytes into a {}-byte file", self.size),
            ));
        }
        self.out
            .extend_from_slice(block.get(..filled).unwrap_or_default());
        Ok(())
    }

    /// The whole plaintext: reads what is left, then checks a compressed file's zlib stream ended with it.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        self.read(0, self.size)?;
        if let Some(z) = &mut self.inflate
            && !z.ended
        {
            // the stream must end here: one more call may only find its end
            let mut extra = [0u8; 1];
            loop {
                if z.used >= z.input.len() {
                    z.input.clear();
                    z.used = 0;
                    if self.cipher.chunk(&mut z.input) == 0 {
                        break;
                    }
                }
                let input = z.input.get(z.used..).unwrap_or_default();
                let step = z
                    .inflater
                    .inflate(input, &mut extra)
                    .map_err(|e| bad(Space::File, self.size, e.to_string()))?;
                z.used = z.used.saturating_add(step.consumed);
                if step.produced > 0 {
                    return Err(bad(
                        Space::File,
                        self.size,
                        format!("the zlib stream holds more than the file's {} bytes", self.size),
                    ));
                }
                if step.ended {
                    z.ended = true;
                    break;
                }
                if step.consumed == 0 {
                    break;
                }
            }
            if !z.ended {
                return Err(bad(
                    Space::File,
                    self.size,
                    "the zlib stream does not end with the file",
                ));
            }
        }
        if self.inflate.is_some() && self.out.len() != self.size {
            return Err(bad(
                Space::File,
                self.size,
                format!("{} bytes inflated for a {}-byte file", self.out.len(), self.size),
            ));
        }
        self.out.truncate(self.size);
        Ok(self.out)
    }
}
