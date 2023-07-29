// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use crate::archive_human_input_file::*;
use crate::output_file_writer::*;
use crate::color_multiplexer::ColorMultiplexer;
use crate::grayscale_recognizer::recognize_grayscale_barcodes;
use base45::decode;

pub struct FileDecoder<'a> {
    file_reader: &'a mut ArchiveHumanInputFile<'a>
}

impl<'a, 'b> FileDecoder<'a> {
    pub fn new(file_reader: &'a mut ArchiveHumanInputFile<'a>) -> FileDecoder<'a> {
        FileDecoder {
            file_reader: file_reader
        }
    }

    pub fn finalize(self) -> FileDecoder<'a> {
        FileDecoder {
            file_reader: self.file_reader
        }
    }

    fn process_decoded_chunk(&mut self, encoded_data: &Vec<u8>, file_writer: &mut OutputFileWriter) {
        // Don't know why there are 4 bytes of junk at the start of this.
        match decode(&std::str::from_utf8(&encoded_data).unwrap()) {
            Ok(data_chunk) => {
                // Skip blank chunks.
                if data_chunk.len() < 1 {
                    return;
                }

                println!("Decoded chunk {:?}", data_chunk);
                let format_version = data_chunk[0];
                if format_version != 1 {
                    panic!("Unsupported format version {}", format_version);
                }

                let page_number = u16::from_be_bytes([data_chunk[1], data_chunk[2]]);
                let barcode_number = u16::from_be_bytes([data_chunk[3], data_chunk[4]]);
                let start_offset = u64::from_be_bytes([0, 0, data_chunk[5], data_chunk[6], data_chunk[7], data_chunk[8], data_chunk[9], data_chunk[10]]);
                let total_length = u64::from_be_bytes([0, 0, data_chunk[11], data_chunk[12], data_chunk[13], data_chunk[14], data_chunk[15], data_chunk[16]]);
                let hash = u32::from_be_bytes([0, data_chunk[17], data_chunk[18], data_chunk[19]]);
                let overhead: usize = 20;
                if start_offset > total_length {
                    //Padding - ignore it.
                    println!("Pure padding - ignoring");
                }
                else if start_offset + (data_chunk.len() - overhead) as u64 > total_length {
                    // Partial chunk.
                    println!("Partial chunk on page {}, barcode number {}, at {}/{} with length {}", page_number, barcode_number, start_offset, total_length, (total_length as usize - start_offset as usize));
                    file_writer.put_chunk(start_offset, &data_chunk[overhead..(total_length as usize - start_offset as usize + overhead)]);
                }
                else {
                    println!("Full chunk on page {}, barcode number {}, at {}/{} with length {}", page_number, barcode_number, start_offset, total_length, (data_chunk.len() - overhead));
                    file_writer.put_chunk(start_offset, &data_chunk[overhead..data_chunk.len()]);
                }
            },
            Err(e) => {
                println!("Decoding error {}: {:?}", e, encoded_data);
            }
        }
    }

    pub fn decode(&mut self, file_writer: &mut OutputFileWriter, color_multiplexer: &mut ColorMultiplexer, adjust_colors: bool) {
        // Just doing this once for now.
        let page_image = self.file_reader.read_page().unwrap();
        if adjust_colors {
            color_multiplexer.palettize_from_image(&page_image);
        }
        let demuxed_images = color_multiplexer.demultiplex_image(&page_image);
        for d in demuxed_images {
            let chunks = recognize_grayscale_barcodes(&d);
            for c in chunks {
                self.process_decoded_chunk(&c, file_writer);
            }
        }
    }
}