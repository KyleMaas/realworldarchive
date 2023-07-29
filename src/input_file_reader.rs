// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use std::fs::File;
use positioned_io::ReadAt;

pub struct InputFileReader<'a> {
    in_file: &'a str,
    file: File,
    len: u64,
    generated_hash: bool,
    file_hash: u32
}

impl<'a> InputFileReader<'a> {
    pub fn new(in_file: &'a str) -> InputFileReader<'a> {
        // TODO: Find a way to idiomatically exclusively lock files in a cross-platform manner, so we are making a lot of assumptions that the file won't change during read.
        let file = File::open(in_file).unwrap();
        let metadata = file.metadata().unwrap();
        let len = metadata.len();

        InputFileReader {
            in_file,
            file: file,
            len,
            generated_hash: false,
            file_hash: 0
        }
    }

    pub fn finalize(self) -> InputFileReader<'a> {
        InputFileReader {
            in_file: self.in_file,
            file: self.file,
            len: self.len,
            generated_hash: self.generated_hash,
            file_hash: self.file_hash
        }
    }

    /// Returns the total length of the file
    pub fn stream_len(&self) -> u64 {
        return self.len;
    }

    /// Returns the hash of the overall file
    pub fn file_hash(&mut self) -> u32 {
        if !self.generated_hash {
            // This can be parallellized later.  For now, we're just going to do this as a loop for simplicity.
            let mut start_pos = 0;
            let mut crc32s: Vec<u8> = Vec::new();
            const CHUNK_SIZE:usize = 1024 * 1024;
            let mut buf:[u8; CHUNK_SIZE] = [0; CHUNK_SIZE];
            while start_pos < self.stream_len() {
                let num_bytes = self.get_chunk(start_pos, &mut buf);
                if num_bytes < CHUNK_SIZE {
                    // Pad the last chunk out to a full block with zeroes
                    buf[num_bytes..CHUNK_SIZE].fill(0);
                }
                let hash = crc32fast::hash(&buf);
                let bytes = hash.to_be_bytes();
                crc32s.push(bytes[0]);
                crc32s.push(bytes[1]);
                crc32s.push(bytes[2]);
                crc32s.push(bytes[3]);
                start_pos += 1024 * 1024;
            }
            self.file_hash = crc32fast::hash(&crc32s);
            self.generated_hash = true;
        }

        self.file_hash
    }

    /// Reads a chunk of data from the file and returns the actual read length
    pub fn get_chunk(&self, pos: u64, buf: &mut [u8]) -> usize {
        return self.file.read_at(pos, buf).unwrap();
    }
}