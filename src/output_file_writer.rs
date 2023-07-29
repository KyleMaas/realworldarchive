// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use std::fs::File;
use positioned_io::WriteAt;

pub struct OutputFileWriter<'a> {
    out_file: &'a str,
    file: File
}

impl<'a> OutputFileWriter<'a> {
    pub fn new(out_file: &'a str) -> OutputFileWriter<'a> {
        // TODO: Find a way to idiomatically exclusively lock files in a cross-platform manner, so we are making a lot of assumptions that the file won't change during read.
        let file = File::create(out_file).unwrap();

        OutputFileWriter {
            out_file,
            file: file
        }
    }

    pub fn finalize(self) -> OutputFileWriter<'a> {
        OutputFileWriter {
            out_file: self.out_file,
            file: self.file
        }
    }

    // Writes a chunk of data to the file.
    pub fn put_chunk(&mut self, pos: u64, buf: &[u8]) -> usize {
        return self.file.write_at(pos, buf).unwrap();
    }
}