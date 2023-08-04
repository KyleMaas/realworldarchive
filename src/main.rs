// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

extern crate clap;
use clap::{Arg, App};
extern crate image;
//extern crate rqrr;
extern crate bardecoder;
extern crate env_logger;
extern crate glob;
use image::RgbImage;

mod stress_test_page;
mod archive_human_output_file;
mod archive_human_input_file;
mod grayscale_recognizer;
mod data_file;
mod page_barcode_packer;
mod color_multiplexer;
mod file_decoder;
use stress_test_page::StressTestPage;
use archive_human_output_file::{OutputFormat, ArchiveHumanOutputFile};
use archive_human_input_file::ArchiveHumanInputFile;
use data_file::DataFile;
use page_barcode_packer::{BarcodeFormat, PageBarcodePacker};
use color_multiplexer::ColorMultiplexer;
use file_decoder::FileDecoder;
use glob::glob;
use std::sync::Arc;

fn validate_integer(v: String) -> Result<(), String> {
    match v.parse::<u16>() {
        Ok(_n) => return Ok(()),
        Err(_) => return Err(String::from("Value given was not a positive integer."))
    }
}

fn validate_positive_float(v: String) -> Result<(), String> {
    match v.parse::<f32>() {
        Ok(n) => if n >= 0.0 { return Ok(()); } else { return Err(String::from("Value given was negative."))},
        Err(_) => return Err(String::from("Value given was not numeric."))
    }
}

fn main() {
    env_logger::init();

    let matches = App::new("Real World Archive")
                    .version("0.0.1")
                    .author("Kyle Maas <kylemaasdev@gmail.com>")
                    .about("Archives data to a format suitable for printing or engraving.")
                    .arg(Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("File or directory to read input from.  Required unless running a stress test in encode mode.")
                        .takes_value(true)
                        .required_unless_all(&["stresstest", "encode"])
                        .display_order(1))
                    .arg(Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("File or directory to place output in.  Required unless running a stress test in decode mode.")
                        .takes_value(true)
                        .required_unless_all(&["stresstest", "decode"])
                        .display_order(2))
                    .arg(Arg::with_name("format")
                        .short("f")
                        .long("format")
                        .help("Output format to use.  Currently only \"png\" is supported, and is the default output format.")
                        .takes_value(true)
                        .possible_values(&["png"])
                        .default_value("png"))
                    .arg(Arg::with_name("units")
                        .short("u")
                        .long("units")
                        .help("Unit system to use for measurements.  Defaults to \"in\"")
                        .takes_value(true)
                        .possible_values(&["in", "mm", "px"])
                        .default_value("in"))
                    .arg(Arg::with_name("pagewidth")
                        .short("w")
                        .long("width")
                        .help("Page width, in real world units.  Defaults to \"8.5\"")
                        .takes_value(true)
                        .validator(validate_positive_float)
                        .default_value("8.5"))
                    .arg(Arg::with_name("pageheight")
                        .short("h")
                        .long("height")
                        .help("Page height, in real world units.  Defaults to \"11\"")
                        .takes_value(true)
                        .validator(validate_positive_float)
                        .default_value("11"))
                    .arg(Arg::with_name("margins")
                        .short("m")
                        .long("margins")
                        .help("Margins, specified as a space-separated list of top, right, bottom, left.  Defaults to \"0.25 0.25 0.5 0.25\"")
                        .takes_value(true)
                        .default_value("0.25 0.25 0.5 0.25"))
                    .arg(Arg::with_name("dpi")
                        .short("D")
                        .long("dpi")
                        .help("Target DPI.  Defaults to \"300\"")
                        .validator(validate_integer)
                        .default_value("300"))
                    .arg(Arg::with_name("colors")
                        .short("c")
                        .long("colors")
                        .help("Maximum number of colors.  Defaults to \"2\" for monochrome")
                        .validator(validate_integer)
                        .default_value("2"))
                    .arg(Arg::with_name("decode")
                        .short("d")
                        .long("decode")
                        .help("Use this to decode the given filename.  Either encode or decode must be specified.")
                        .required_unless("encode")
                        .conflicts_with("encode")
                        .display_order(1))
                    .arg(Arg::with_name("encode")
                        .short("e")
                        .long("encode")
                        .help("Encode to the given filename as output.  Either encode or decode must be specified.")
                        .required_unless("decode")
                        .conflicts_with("decode")
                        .display_order(2))
                    .arg(Arg::with_name("parity")
                        .short("p")
                        .long("parity")
                        .help("Number of pages of parity to generate.  This equates to the number of full pages which can be lost from the rest of the document.  Defaults to \"0\"")
                        .default_value("0"))
                    .arg(Arg::with_name("stresstest")
                        .short("t")
                        .long("stresstest")
                        .help("Generate a stress test"))
                    .get_matches();
    let width = matches.value_of("pagewidth").unwrap();
    let height = matches.value_of("pageheight").unwrap();
    let dpi = matches.value_of("dpi").unwrap();
    let colors = matches.value_of("colors").unwrap();
    let format = OutputFormat::PNG; //matches.value_of("format").unwrap().to_lowercase();
    if matches.is_present("encode") {
        // Encode.
        let out_file = matches.value_of("output").unwrap();
        if matches.is_present("stresstest") {
            // Generate a stress test page.
            let header = "Stress Test - {{dpi}} DPI, {{total_overlay_colors}}x Color Packing";
            let writer = ArchiveHumanOutputFile::new(out_file, format)
                .size(width.parse::<f32>().unwrap(), height.parse::<f32>().unwrap())
                .dpi(dpi.parse::<u16>().unwrap())
                .document_header(&header)
                .document_footer("Scan to test limits of your printing and scanning process")
                .total_pages(1)
                .finalize();
            let stress_test = StressTestPage::new()
                .finalize();
            stress_test.encode(&writer);
        }
        else {
            // Encode normal data.
            let in_file = matches.value_of("input").unwrap();
            let out_file = matches.value_of("output").unwrap();
            let mut file_reader = DataFile::new(in_file, false).finalize();
            let color_multiplexer = ColorMultiplexer::new(colors.parse::<u8>().unwrap()).finalize();
            let header = in_file;
            let mut writer = ArchiveHumanOutputFile::new(out_file, format)
                .size(width.parse::<f32>().unwrap(), height.parse::<f32>().unwrap())
                .dpi(dpi.parse::<u16>().unwrap())
                .document_header(&header)
                .document_footer("Page {{page_num}}/{{total_pages}}")
                .colors(color_multiplexer.get_rgb())
                .finalize();
            if color_multiplexer.num_colors() > 2 {
                writer.set_document_footer("Page {{page_num}}/{{total_pages}} - {{total_overlay_colors}} Colors");
            }
            let (w, h) = writer.get_barcode_image_size();
            let mut barcode_packer = PageBarcodePacker::new(w, h, BarcodeFormat::QR)
                .color_multiplexer(color_multiplexer)
                .finalize();
            println!("Maximum bytes per page: {}", barcode_packer.data_bytes_per_page());

            // Let's see if we can optimize that to expand barcodes to their maximum size.
            // This is almost always only going to be possible when the document itself can very easily fit on a single page.
            let mut start_offset: u64 = 0;
            let total_len = file_reader.stream_len();
            println!("Total file length: {}", total_len);
            let max_block_size = barcode_packer.data_bytes_per_page() as u64;
            let total_pages_at_max_data_rate = ((total_len + (max_block_size - 1)) / max_block_size) as u16; // See https://www.reddit.com/r/rust/comments/bk7v15/my_next_favourite_way_to_divide_integers_rounding/
            let min_bytes_per_page = ((total_len + (total_pages_at_max_data_rate as u64 - 1)) / (total_pages_at_max_data_rate as u64)) as u32; // Redividing this so we can round properly.
            while barcode_packer.repack_barcodes_for_page_length(min_bytes_per_page) {};
            println!("Ideal bytes per page: {}", barcode_packer.data_bytes_per_page());
            
            // Write to image files as a quick test.
            let mut out_image = RgbImage::new(w, h);
            println!("Checking for data bytes per page");
            let block_size = barcode_packer.data_bytes_per_page() as u64;
            let total_pages = ((total_len + (block_size - 1)) / block_size) as u16;
            writer.set_total_pages(total_pages);
            println!("Block size: {}", block_size);
            let mut block_buffer_vec: Vec<u8> = vec![];
            for _b in 0..block_size {
                block_buffer_vec.push(0);
            }
            let block_buffer: &mut [u8] = block_buffer_vec.as_mut_slice();
            let file_checksum = file_reader.file_hash();
            println!("File checksum: {}", file_checksum);
            while start_offset < total_len {
                // Page numbers are 1-based to match what's shown to the user.
                let page_number = ((start_offset / block_size) as u16) + 1;
                println!("Generating page {}", page_number);
                let actually_read = file_reader.get_chunk(start_offset, block_buffer);
                if actually_read < block_size as usize {
                    // Pad the data.
                    for i in actually_read..(block_size as usize) {
                        block_buffer[i] = 0;
                    }
                }
                //let last_page = page_number == total_pages;
                barcode_packer.encode(&mut out_image, page_number, false, 0, file_checksum, start_offset, total_len, block_buffer);
                //let numbered_filename = format!("{}{}.png", out_file, page_number);
                //println!("Writing to {}", numbered_filename);
                writer.write_page(&out_image, page_number);
                //out_image.save(numbered_filename).unwrap();
                start_offset += block_size;
            }
        }
    }
    else {
        // Decode.
        let in_file = matches.value_of("input").unwrap();
        if matches.is_present("stresstest") {
            // Decode a stress test page.
            let reader = ArchiveHumanInputFile::new(in_file, format)
                .finalize();
            let stress_test = StressTestPage::new()
                .finalize();
            stress_test.decode(&reader);
        }
        else {
            // Decode normal data.
            let out_file = matches.value_of("output").unwrap();
            let mut file_writer = DataFile::new(out_file, true).finalize();
            let mut color_multiplexer = ColorMultiplexer::new(colors.parse::<u8>().unwrap()).finalize();
            let in_files_glob = glob(in_file).expect("Failed to read glob pattern");
            let mut first_file = true;
            let mut chunk_info = vec![];
            for f in in_files_glob {
                match f {
                    Ok(filename) => {
                        println!("Decoding file {}", filename.to_str().unwrap());
                        let mut one_in_file = ArchiveHumanInputFile::new(filename.to_str().unwrap(), format);
                        let mut decoder = FileDecoder::new(&mut one_in_file).finalize();

                        // If it's the first page, go ahead and re-palettize the color multiplexer based on the colors found in it, to account for color distortion in the printing/scanning process.
                        let mut chunks_on_page = decoder.decode(&mut file_writer, &mut color_multiplexer, first_file);
                        chunk_info.append(&mut chunks_on_page);
                        first_file = false;
                    },
                    Err(e) => println!("{:?}", e)
                }
            }
            //println!("Decoded using {} color planes", color_multiplexer.num_planes());

            // Make sure the has matches.
            println!("Checking file integrity...");
            if chunk_info.len() < 1 {
                panic!("Could not find even a single barcode to read");
            }
            if chunk_info[0].total_length != file_writer.stream_len() {
                panic!("Output file length {} does not match the expected {}", file_writer.stream_len(), chunk_info[0].total_length);
            }
            let hash = file_writer.file_hash() & 0x00ffffff;
            if chunk_info[0].hash != hash {
                panic!("File checksum {} did not match the expected {}", hash, chunk_info[0].hash);
            }
            println!("File passed integrity checks!");
        }
    }
}
