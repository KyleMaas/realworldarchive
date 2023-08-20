// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

extern crate clap;
use clap::{Command, Arg, ArgAction};
extern crate image;
//extern crate rqrr;
extern crate bardecoder;
extern crate env_logger;
extern crate glob;
extern crate reed_solomon_erasure;
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
use page_barcode_packer::{BarcodeFormat, PageBarcodePacker, make_constant_damage_map, make_radial_damage_map};
use color_multiplexer::ColorMultiplexer;
use file_decoder::FileDecoder;
use glob::glob;
use reed_solomon_erasure::galois_8::ReedSolomon;

fn main() {
    env_logger::init();

    let matches = Command::new("Real World Archive")
                    .version("0.0.1")
                    .author("Kyle Maas <kylemaasdev@gmail.com>")
                    .about("Archives data to a format suitable for printing or engraving.")
                    .arg(Arg::new("input")
                        .short('i')
                        .long("input")
                        .help("File or directory to read input from.  Required unless running a stress test in encode mode.")
                        .required_unless_present_all(&["stresstest", "encode"])
                        .display_order(1))
                    .arg(Arg::new("output")
                        .short('o')
                        .long("output")
                        .help("File or directory to place output in.  Required unless running a stress test in decode mode.")
                        .required_unless_present_all(&["stresstest", "decode"])
                        .display_order(2))
                    .arg(Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Output format to use.  Currently only \"png\" is supported, and is the default output format.")
                        .value_parser(["png"])
                        .default_value("png"))
                    .arg(Arg::new("units")
                        .short('u')
                        .long("units")
                        .help("Unit system to use for measurements.  Defaults to \"in\"")
                        .value_parser(["in", "mm", "px"])
                        .default_value("in"))
                    .arg(Arg::new("pagewidth")
                        .short('W')
                        .long("width")
                        .help("Page width, in real world units.  Defaults to \"8.5\"")
                        .value_parser(clap::value_parser!(f32))
                        .default_value("8.5"))
                    .arg(Arg::new("pageheight")
                        .short('H')
                        .long("height")
                        .help("Page height, in real world units.  Defaults to \"11\"")
                        .value_parser(clap::value_parser!(f32))
                        .default_value("11"))
                    .arg(Arg::new("margins")
                        .short('m')
                        .long("margins")
                        .help("Margins, specified as a space-separated list of top, right, bottom, left.  Defaults to \"0.25 0.25 0.5 0.25\"")
                        .default_value("0.25 0.25 0.5 0.25"))
                    .arg(Arg::new("dpi")
                        .short('D')
                        .long("dpi")
                        .help("Target DPI.  Defaults to \"300\"")
                        .value_parser(clap::value_parser!(u16).range(1..4801))
                        .default_value("300"))
                    .arg(Arg::new("colors")
                        .short('c')
                        .long("colors")
                        .help("Maximum number of colors.  Defaults to \"2\" for monochrome")
                        .value_parser(clap::value_parser!(u8).range(2..))
                        .default_value("2"))
                    .arg(Arg::new("ecfunction")
                        .long("ecfunction")
                        .help("Error correction function for how much error correction to use for each barcode depending on its position on the page.  Defaults to \"radial\" to skew error correction so there is less in the center of the page and more toward the corners but can be set to \"constant\" for a constant level of error correction across the entire page")
                        .value_parser(["constant", "radial"])
                        .default_value("radial"))
                    .arg(Arg::new("ecmin")
                        .long("ecmin")
                        .help("Minimum percentage of error correction - just the number [0..100].  Please note this is not the amount of a barcode which can be lost and recovered but a percentage of the range we can run on.  For example, QR codes have a \"0\" level of 7% error corraction and \"100\" level of 30% of data which can be recovered.  If the constant error correction function is used, this is the amount used over the whole page.  Defaults to \"25\"")
                        .value_parser(clap::value_parser!(u8).range(0..101))
                        .default_value("25"))
                    .arg(Arg::new("ecmax")
                        .long("ecmax")
                        .help("Maximum percentage of error correction - just the number [0..100].  Please note this is not the amount of a barcode which can be lost and recovered but a percentage of the range we can run on.  For example, QR codes have a \"0\" level of 7% error corraction and \"100\" level of 30% of data which can be recovered.  Only applicable in non-constant error correction functions.  Defaults to \"100\"")
                        .value_parser(clap::value_parser!(u8).range(0..101))
                        .default_value("100"))
                    .arg(Arg::new("decode")
                        .short('d')
                        .long("decode")
                        .help("Use this to decode the given filename.  Either encode or decode must be specified.")
                        .required_unless_present("encode")
                        .conflicts_with("encode")
                        .action(ArgAction::SetTrue)
                        .display_order(1))
                    .arg(Arg::new("encode")
                        .short('e')
                        .long("encode")
                        .help("Encode to the given filename as output.  Either encode or decode must be specified.")
                        .required_unless_present("decode")
                        .conflicts_with("decode")
                        .action(ArgAction::SetTrue)
                        .display_order(2))
                    .arg(Arg::new("parity")
                        .short('p')
                        .long("parity")
                        .help("Number of pages of parity to generate in the range [0..63].  This equates to the number of full pages which can be lost from the rest of the document.  Defaults to \"0\"")
                        .value_parser(clap::value_parser!(u8).range(0..64))
                        .default_value("0"))
                    .arg(Arg::new("stresstest")
                        .short('t')
                        .long("stresstest")
                        .action(ArgAction::SetTrue)
                        .help("Generate a stress test"))
                    .get_matches();
    let format = OutputFormat::PNG; //matches.value_of("format").unwrap().to_lowercase();
    let colors = *matches.get_one::<u8>("colors").unwrap();
    if matches.get_flag("encode") {
        // Encode.
        let width = *matches.get_one::<f32>("pagewidth").unwrap();
        let height = *matches.get_one::<f32>("pageheight").unwrap();
        let dpi = *matches.get_one::<u16>("dpi").unwrap();
        let out_file = matches.get_one::<String>("output").unwrap().as_str();
        if matches.get_flag("stresstest") {
            // Generate a stress test page.
            let header = "Stress Test - {{dpi}} DPI, {{total_overlay_colors}}x Color Packing";
            let writer = ArchiveHumanOutputFile::new(out_file, format)
                .size(width, height)
                .dpi(dpi)
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
            let in_file = matches.get_one::<String>("input").unwrap();
            let out_file = matches.get_one::<String>("output").unwrap();
            let mut file_reader = DataFile::new(in_file, false).finalize();
            let color_multiplexer = ColorMultiplexer::new(colors).finalize();
            let parity_pages = *matches.get_one::<u8>("parity").unwrap();
            let damage_function = matches.get_one::<String>("ecfunction").unwrap().as_str();
            let ec_min = *matches.get_one::<u8>("ecmin").unwrap() as f32 / 100.0;
            let ec_max = *matches.get_one::<u8>("ecmax").unwrap() as f32 / 100.0;
            let header = in_file;
            let mut writer = ArchiveHumanOutputFile::new(out_file, format)
                .size(width, height)
                .dpi(dpi)
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
                .damage_likelihood_map(if damage_function == "constant" { make_constant_damage_map(ec_min) } else { make_radial_damage_map(ec_min, ec_max) })
                .finalize();
            //println!("Maximum bytes per page: {}", barcode_packer.data_bytes_per_page());

            // Let's see if we can optimize that to expand barcodes to their maximum size.
            // This is almost always only going to be possible when the document itself can very easily fit on a single page.
            let mut start_offset: u64 = 0;
            let total_len = file_reader.stream_len();
            //println!("Total file length: {}", total_len);
            let max_block_size = barcode_packer.data_bytes_per_page() as u64;
            let total_pages_at_max_data_rate = ((total_len + (max_block_size - 1)) / max_block_size) as u16; // See https://www.reddit.com/r/rust/comments/bk7v15/my_next_favourite_way_to_divide_integers_rounding/
            let min_bytes_per_page = ((total_len + (total_pages_at_max_data_rate as u64 - 1)) / (total_pages_at_max_data_rate as u64)) as u32; // Redividing this so we can round properly.
            while barcode_packer.repack_barcodes_for_page_length(min_bytes_per_page) {};
            //println!("Ideal bytes per page: {}", barcode_packer.data_bytes_per_page());
            
            // Write to image files as a quick test.
            let mut out_image = RgbImage::new(w, h);
            //println!("Checking for data bytes per page");
            let block_size = barcode_packer.data_bytes_per_page() as u64;
            let total_data_pages = ((total_len + (block_size - 1)) / block_size) as u16;
            let total_pages = total_data_pages + parity_pages as u16;
            writer.set_total_pages(total_pages);
            //println!("Block size: {}", block_size);
            let mut block_buffer_vec: Vec<u8> = vec![];
            for _b in 0..block_size {
                block_buffer_vec.push(0);
            }
            let block_buffer: &mut [u8] = block_buffer_vec.as_mut_slice();
            let file_checksum = file_reader.file_hash();
            //println!("File checksum: {}", (file_checksum & 0x00ffffff));
            while start_offset < total_len {
                // Page numbers are 1-based to match what's shown to the user.
                let page_number = ((start_offset / block_size) as u16) + 1;
                println!("Generating page {}...", page_number);
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

            // Add parity pages.
            if parity_pages > 0 {
                println!("Calculating parity...");

                // First, go through and calculate parity to buffers.
                let mut parity_data: Vec<Vec<u8>> = vec![];
                for _p in 0..parity_pages {
                    parity_data.push(vec![]);
                }

                // Do parity calculation on blocks of bytes at a time rather than one at a time, to cut down on the number of I/O ops.
                // This is how many bytes of parity we're calculating at a time.
                // TODO: Vary the size of the read blocks based on how large the actual file is - the main point here is to split it up so we're not reading the entire file into memory to do parity, so if the file is short, this block size could be larger and still not use a ton of RAM.
                let bytes_per_read = 256;
                let mut parity_block_start_offset = 0;
                while parity_block_start_offset < block_size {
                    // Skip through the document on a stride of the page size and an offset of parity_block_start_offset.
                    let mut data_read_block_start_offset = parity_block_start_offset;
                    // This variable will hold the reformatted data to run parity on - for example:
                    // [0][0] will be byte 0 of the first page of the document, [0][1] will be the first byte of barcode #0 on the next page, etc.
                    // [1][0] will be byte 1 of the first page of the document, [1][1] will be the second byte of barcode #0 on the next page, etc.
                    // Each byte has to be wrapped in another Vec due to the way that reed-solomon-erasure works with shards as slices.
                    let mut data_streams_for_parity: Vec<Vec<Vec<u8>>> = vec![];
                    for _b in 0..bytes_per_read {
                        data_streams_for_parity.push(vec![]);
                    }
                    while data_read_block_start_offset < total_len {
                        // Read one block worth of data and reformat it into the parity block buffers.
                        // By prefilling this with 0's, we're dealing with the padding issue for parity calculations.
                        let mut block_buffer = vec![];
                        for _b in 0..bytes_per_read {
                            block_buffer.push(0);
                        }
                        file_reader.get_chunk(data_read_block_start_offset, block_buffer.as_mut_slice());
                        for b in 0..bytes_per_read {
                            data_streams_for_parity[b].push(vec![block_buffer[b]]);
                        }

                        // Advance to the next page.
                        data_read_block_start_offset += block_size;
                    }

                    // Pad these to length so even if we go off the end of the document reading we can still calculate parity.
                    for b in 0..bytes_per_read {
                        for _c in (data_streams_for_parity[b].len() as u16)..total_data_pages {
                            data_streams_for_parity[b].push(vec![0]);
                        }
                    }

                    // Calculate parity.
                    // As a reminder, we're doing this such that each parity byte 0 protects data byte 0 of each page.
                    // Because of this, data_streams_for_parity[0].len() should always be the same as the number of data pages.
                    // We've ensured this above by padding to length.
                    /*if total_data_pages as usize != data_streams_for_parity[0].len() {
                        panic!("Somehow we ended up with a stream of bytes for parity with length {} which is not the same as the number of data pages {} (starting offset {})", data_streams_for_parity[0].len(), total_data_pages, parity_block_start_offset);
                    }*/
                    let num_data_bytes = total_data_pages as usize;
                    let enc = ReedSolomon::new(num_data_bytes, parity_pages as usize).unwrap();
                    for b in 0..bytes_per_read {
                        // Add placeholders for the parity bytes.
                        for _p in 0..parity_pages {
                            data_streams_for_parity[b].push(vec![0]);
                        }
                        enc.encode(data_streams_for_parity[b].as_mut_slice()).unwrap();
                        for p in 0..(parity_pages as usize) {
                            let parity_byte = data_streams_for_parity[b][num_data_bytes + p][0];
                            parity_data[p].push(parity_byte);
                        }
                    }

                    // Advance our read block position.
                    parity_block_start_offset += bytes_per_read as u64;
                }

                // Write out the parity pages.
                for p in 0..parity_pages {
                    println!("Generating parity page {}...", (p + 1));
                    let page_number = total_data_pages + p as u16 + 1;
                    //println!("Parity bytes: {:?}", parity_data[p as usize]);
                    barcode_packer.encode(&mut out_image, page_number, true, p as u8, file_checksum, 0, total_len, &parity_data[p as usize][0..block_size as usize]);
                    writer.write_page(&out_image, page_number);
                }
            }
        }
    }
    else {
        // Decode.
        let in_file: &String = matches.get_one("input").unwrap();
        if matches.get_flag("stresstest") {
            // Decode a stress test page.
            let reader = ArchiveHumanInputFile::new(in_file, format)
                .finalize();
            let stress_test = StressTestPage::new()
                .finalize();
            stress_test.decode(&reader);
        }
        else {
            // Decode normal data.
            let out_file: &String = matches.get_one("output").unwrap();
            let mut file_writer = DataFile::new(out_file, true).finalize();
            let mut color_multiplexer = ColorMultiplexer::new(colors).finalize();
            let in_files_glob = glob(in_file).expect("Failed to read glob pattern");
            let mut first_file = true;
            let mut chunk_info = vec![];
            let mut parity_buffer: Vec<Vec<u8>> = vec![]; // Each element is a vector of bytes for that page.
            for f in in_files_glob {
                match f {
                    Ok(filename) => {
                        println!("Decoding file {}", filename.to_str().unwrap());
                        let mut one_in_file = ArchiveHumanInputFile::new(filename.to_str().unwrap(), format);
                        let mut decoder = FileDecoder::new(&mut one_in_file).finalize();

                        // If it's the first page, go ahead and re-palettize the color multiplexer based on the colors found in it, to account for color distortion in the printing/scanning process.
                        let mut chunks_on_page = decoder.decode(&mut file_writer, &mut parity_buffer, &mut color_multiplexer, first_file);
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

            // Sort the chunks by start offset for easier detection later.
            // TODO: Use something other than bubble sort.
            for i in 0..chunk_info.len() - 1 {
                for j in (i + 1)..chunk_info.len() {
                    if chunk_info[j].start_offset < chunk_info[i].start_offset {
                        let temp = chunk_info[j];
                        chunk_info[j] = chunk_info[i];
                        chunk_info[i] = temp;
                    }
                }
            }

            // Look over all the byte ranges and make sure we've constructed the whole file, and if not, try to detect what we're missing.
            let mut ranges: Vec<[u64; 2]> = vec![]; // Array of start offsets and end offsets, merged.
            let mut page_numbers_we_have: Vec<u16> = vec![];
            for i in 0..chunk_info.len() {
                // Skip parity for now.
                if chunk_info[i].is_parity {
                    continue;
                }

                // Skip barcodes which are purely padding.
                if chunk_info[i].start_offset > chunk_info[0].total_length {
                    continue;
                }

                // Find the overlapping ranges.
                let mut adjoins_or_overlaps = vec![];
                let mut start_offset = chunk_info[i].start_offset;
                let mut end_offset = start_offset + chunk_info[i].length as u64;
                for j in 0..ranges.len() {
                    if ranges[j][0] <= end_offset && ranges[j][1] >= start_offset {
                        // New range overlaps.
                        //println!("Range {:?} overlaps with chunk {:?}", ranges[j], chunk_info[i]);
                        adjoins_or_overlaps.push(j);
                        start_offset = start_offset.min(ranges[j][0]);
                        end_offset = end_offset.max(ranges[j][1]);
                    }
                    /*else {
                        println!("Range {:?} does not overlap {} to {}", ranges[j], start_offset, end_offset);
                    }*/
                }

                // Remove all ranges which overlap in favor of the joined one.
                adjoins_or_overlaps.sort();
                for r in (0..adjoins_or_overlaps.len()).rev() {
                    ranges.remove(adjoins_or_overlaps[r]);
                }

                // Add this new range.
                //println!("New range: {} to {}", start_offset, end_offset);
                ranges.push([start_offset, end_offset]);
                //println!("Ranges is now: {:?}", ranges);
                page_numbers_we_have.push(chunk_info[i].page_number);
            }
            page_numbers_we_have.sort();
            page_numbers_we_have.dedup();

            // Sort the ranges by starting offset.
            for i in 0..ranges.len() - 1 {
                for j in (i + 1)..ranges.len() {
                    if ranges[j][0] < ranges[i][0] {
                        let temp = ranges[j];
                        ranges[j] = ranges[i];
                        ranges[i] = temp;
                    }
                }
            }

            //println!("Ranges we have: {:?}", ranges);
            let mut missing_ranges = vec![];
            if ranges[0][0] != 0 {
                missing_ranges.push([0, ranges[0][0]]);
            }
            for i in 0..ranges.len() - 1 {
                missing_ranges.push([ranges[i][1], ranges[i + 1][0]]);
            }

            if chunk_info[0].total_length != ranges[ranges.len() - 1][1] {
                // We're missing a chunk at the end.
                // Add it to the list so we can attempt recovery.
                missing_ranges.push([ranges[ranges.len() - 1][1], chunk_info[0].total_length]);
            }
            if missing_ranges.len() > 0 {
                println!("Missing chunks...attempting recovery...");
                
                // First, we need to figure out how large a page is.
                let mut page_size = 0;
                for c in 0..chunk_info.len() - 1 {
                    if chunk_info[c].is_parity {
                        continue;
                    }
                    for d in (c + 1)..chunk_info.len() {
                        if chunk_info[d].is_parity {
                            continue;
                        }

                        // If barcode numbers match and page number only differs by 1, then we have a stride we can work with.
                        if chunk_info[c].barcode_number == chunk_info[d].barcode_number && (chunk_info[c].page_number as i32 - chunk_info[d].page_number as i32).abs() == 1 {
                            page_size = chunk_info[c].start_offset.max(chunk_info[d].start_offset) - chunk_info[c].start_offset.min(chunk_info[d].start_offset);
                        }
                    }
                    if page_size != 0 {
                        break;
                    }
                }
                if page_size == 0 {
                    panic!("Could not find enough information to calculate page size.  Unable to continue with reconstruction of missing chunks.");
                }

                // Attempt to reconstruct each chunk.
                while missing_ranges.len() > 0 {
                    // TODO: Do this in batches instead of one byte at a time, to save on I/O operations.  We're only doing it this way for now for simplicity.
                    for b in missing_ranges[0][0]..missing_ranges[0][1] {
                        // For each missing byte...
                        let total_length = chunk_info[0].total_length;
                        let num_data_pages = (total_length + page_size - 1) / page_size;
                        //let total_length_rounded_up_for_padding = page_size * num_pages;
                        let offset_into_parity = b % page_size;
                        let missing_on_page_number = b / page_size;
                        let mut recovery_byte_buffer: Vec<Option<Vec<u8>>> = vec![];
                        let parity_pages = parity_buffer.len();
                        let dec = ReedSolomon::new(num_data_pages as usize, parity_pages as usize).unwrap();
                        for p in 0..num_data_pages {
                            // Pull the bytes back out of the output file.
                            let start_offset = offset_into_parity + (p * page_size);
                            let mut is_missing = false;
                            for m in 0..missing_ranges.len() {
                                if start_offset >= missing_ranges[m][0] && start_offset < missing_ranges[m][1] {
                                    is_missing = true;
                                    break;
                                }
                            }
                            if is_missing {
                                recovery_byte_buffer.push(None);
                            }
                            else {
                                let mut one_byte = [0 as u8];
                                file_writer.get_chunk(start_offset, &mut one_byte);
                                recovery_byte_buffer.push(Some(vec![one_byte[0]]));
                            }
                        }

                        // Add the parity onto the end and try to reconstruct.
                        for p in 0..parity_pages as usize {
                            recovery_byte_buffer.push(
                                if (offset_into_parity as usize) < parity_buffer[p].len() {
                                    Some(vec![parity_buffer[p][offset_into_parity as usize]])
                                }
                                else {
                                    None
                                });
                        }
                        //println!("Byte buffer before reconstruction: {:?}", recovery_byte_buffer);
                        dec.reconstruct(recovery_byte_buffer.as_mut_slice()).unwrap();
                        //println!("Byte buffer after reconstruction: {:?}", recovery_byte_buffer);
                        let recovered_byte = &recovery_byte_buffer[missing_on_page_number as usize];
                        //println!("Recovered byte: {:?}", recovered_byte);
                        let recovered_byte_value = match recovered_byte {
                            Some(ref x) => x[0],
                            None => panic!("Could not reconstruct byte {} on page {}", b, missing_on_page_number)
                        };
                        println!("Reconstructed byte {} on page {} as {}", b, missing_on_page_number, recovered_byte_value);
                        file_writer.put_chunk(b, &vec![recovered_byte_value]);
                        //panic!("That's as far as we're going to go for now.");
                    }

                    // Now that we've cleared this range, remove it from the unrecoverable list.
                    missing_ranges.remove(0);
                }
            }
            if missing_ranges.len() > 0 {
                for m in 0..missing_ranges.len() {
                    println!("Could not recover bytes {} through {}", missing_ranges[m][0], missing_ranges[m][1]);
                }
                // TODO: Show which pages we're missing where we could recover if we had them.
                panic!("Chunks of the file are missing");
            }

            // Final integrity checks.
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
