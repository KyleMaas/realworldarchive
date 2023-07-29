// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use crate::archive_human_input_file::*;

pub struct PageOrganizer<'a> {
    in_file: &'a str,
    format: OutputFormat,
    cur_page: u16 // 1-based index for page number.
}

impl<'a> ArchiveHumanInputFile<'a> {
    pub fn new(in_file: &'a str, format: OutputFormat) -> ArchiveHumanInputFile<'a> {
        ArchiveHumanInputFile {
            in_file: in_file,
            format: format,
            cur_page: 1
        }
    }

    pub fn finalize(self) -> ArchiveHumanInputFile<'a> {
        ArchiveHumanInputFile {
            in_file: self.in_file,
            format: self.format,
            cur_page: self.cur_page
        }
    }

    pub fn read_page(&self) -> Option<DynamicImage> {
        return Some(image::open(self.in_file).unwrap());
    }
}