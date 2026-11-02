//! Stand-in for the `zstd` stream API: passes bytes through unchanged.
//! Only the size of ops.bin differs from a real zstd build.

pub mod stream {
    pub mod write {
        use std::io::{Result, Write};
        pub struct Encoder<W: Write>(W);
        impl<W: Write> Encoder<W> {
            pub fn new(w: W, _level: i32) -> Result<Self> {
                Ok(Encoder(w))
            }
            pub fn finish(self) -> Result<W> {
                Ok(self.0)
            }
        }
        impl<W: Write> Write for Encoder<W> {
            fn write(&mut self, b: &[u8]) -> Result<usize> {
                self.0.write(b)
            }
            fn flush(&mut self) -> Result<()> {
                self.0.flush()
            }
        }
    }
    pub mod read {
        use std::io::{Read, Result};
        pub struct Decoder<R: Read>(R);
        impl<R: Read> Decoder<R> {
            pub fn new(r: R) -> Result<Self> {
                Ok(Decoder(r))
            }
            pub fn window_log_max(&mut self, _log: u32) -> Result<()> {
                Ok(())
            }
        }
        impl<R: Read> Read for Decoder<R> {
            fn read(&mut self, b: &mut [u8]) -> Result<usize> {
                self.0.read(b)
            }
        }
    }
}
