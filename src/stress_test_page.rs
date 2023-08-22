// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use crate::archive_human_output_file::*;
use crate::archive_human_input_file::*;
use crate::grayscale_recognizer::recognize_grayscale_barcodes;
use crate::color_multiplexer::ColorMultiplexer;
extern crate image;
extern crate regex;
use image::{RgbImage, Rgb};
use image::imageops;
use imageproc::rect::Rect;
use imageproc::drawing::*;
use qrencode::QrCode;
use qrencode::bits::Bits;
use qrencode::types::{Version, EcLevel, Mode};
use regex::Regex;
pub struct StressTestPage {
}

impl<'a> StressTestPage {
    pub fn new() -> StressTestPage {
        StressTestPage {
        }
    }

    pub fn finalize(self) -> StressTestPage {
        StressTestPage {
        }
    }

    fn largest_qrcode_version_for_width(target_size: i16) -> i16 {
        for x in (1..41).rev() {
            let barcode_width = Version::Normal(x).width();
            //println!("Barcode width for version {} is {}", x, barcode_width);
            if barcode_width <= target_size {
                return x;
            }
        }
        return 1;
    }

    fn generate_barcode_filling_bits(qrcode_version: Version, ec_level: EcLevel, message: &str) -> Bits {
        let mut bits = Bits::new(qrcode_version);
        let max_bits = bits.max_len(ec_level).unwrap();
        //println!("Max bits: {}", max_bits);
        let metadata_bits = Mode::Byte.length_bits_count(qrcode_version) + 4 + qrcode_version.mode_bits_count();
        //println!("Metadata bits: {}", metadata_bits);
        let bytes_to_generate = ((max_bits - metadata_bits) / 8) - 1; // null at the end
        //println!("Bytes to generate: {}", bytes_to_generate);
        //let mut test_string = format!("{:width$}", message.repeat(bytes_to_generate / message.len()), width=bytes_to_generate);
        let mut test_string = message.repeat((bytes_to_generate / message.len()) + 1);
        test_string.truncate(bytes_to_generate);
        //println!("Generated test string: {}", test_string);
        //println!("Length of test string: {}", test_string.len());
        let byte_array = test_string.as_bytes();
        bits.push_byte_data(byte_array).unwrap();
        bits.push_terminator(ec_level).unwrap();
        bits
    }

    pub fn encode(&self, writer: &ArchiveHumanOutputFile, max_color_multiplexer: &ColorMultiplexer) {
        // Maximum DPI will be native resolution.  Each successive decrease in resolution will be by half, resulting in full pixels.
        let barcode_image_size = writer.get_barcode_image_size();
        let full_dpi = writer.get_dpi();

        // Build an output page.
        let mut out_image = RgbImage::new(barcode_image_size.0, barcode_image_size.1);
        draw_filled_rect_mut(&mut out_image, Rect::at(0, 0).of_size(barcode_image_size.0, barcode_image_size.1), Rgb([255, 255, 255]));

        // Figure out sizes for each barcode.
        // We want to aim for 1/3 of the height (minus a quiet zone) and fill the with a barcode.
        // Barcodes are padded to fill the space so we can get the most out of the error rate information.
        let quiet_zone = 8; // 8 pixels at full resolution, since the lower-DPI barcodes require larger-pixel quiet zones to work.
        let large_barcode_height = (((barcode_image_size.1 - quiet_zone) / 4) - quiet_zone) as i16;
        //println!("Maximum height: {}", large_barcode_height);
        let largest_barcode_version = StressTestPage::largest_qrcode_version_for_width(large_barcode_height);
        println!("Largest barcode version: {}", largest_barcode_version);
        let max_color_bits_to_test = max_color_multiplexer.num_planes();
        let ec_level = EcLevel::H;
        for num_colors_bits in 1..(max_color_bits_to_test + 1) {
            let x = (barcode_image_size.0 / max_color_bits_to_test as u32) * (num_colors_bits - 1) as u32;
            let num_colors = (2 as u8).pow(num_colors_bits as u32);
            let multiplexer = ColorMultiplexer::new(num_colors);
            for y in 0..4 {
                // Fill up the size of QR we're generating.
                let color_barcodes:Vec<RgbImage> = (0..num_colors_bits).map(|c:u8| {
                    let qrcode_version = Version::Normal(largest_barcode_version >> y);
                    let dpi = full_dpi >> y;
                    let color_description_long = format!("{} Colors, color #{}", num_colors, (c + 1));
                    println!("Generating {} barcode at {} DPI", color_description_long, dpi);
                    let space_for_color = String::from("=").repeat((c + 1) as usize);
                    let message = format!("{} Test at {} DPI in {} =====", space_for_color, dpi, color_description_long);
                    let bits = StressTestPage::generate_barcode_filling_bits(qrcode_version, ec_level, &message);

                    // Generate the QR code.
                    let code = QrCode::with_bits(bits, ec_level).unwrap();
                    code.render::<Rgb<u8>>().module_dimensions(1 << y, 1 << y).quiet_zone(false).build()
                }).collect::<Vec<RgbImage>>();
                let code_image = multiplexer.multiplex_planes(color_barcodes);
                imageops::overlay(&mut out_image, &code_image, x as i64, (((y * large_barcode_height) as u32) + quiet_zone) as i64);
            }
        }

        // Feed it to the writer.
        writer.write_page(&out_image, 0);
    }

    pub fn decode(&self, reader: &ArchiveHumanInputFile, max_color_multiplexer: &ColorMultiplexer) {
        println!("Reading image");
        let image = reader.read_page().unwrap();

        // For each bitplane depth, demultiplex and try decoding.
        let re = Regex::new(r"= Test at ([0-9]+) DPI in ([0-9]+) Colors, color #([0-9]+)").unwrap();
        let max_color_bits_to_test = max_color_multiplexer.num_planes();
        for num_colors_bits in (1..(max_color_bits_to_test + 1)).rev() {
            let mut located_all = true;
            let num_colors = (2 as u8).pow(num_colors_bits as u32);
            println!("Attempting to decode at {} colors...", num_colors);
            let multiplexer = ColorMultiplexer::new(num_colors);
            //println!("- Detecting colors...");
            //multiplexer.palettize_from_image(&image);
            println!("- Demultiplexing...");
            let bit_planes = multiplexer.demultiplex_image(&image);
            let mut found_barcodes = vec![];
            let mut dpis_found = vec![];
            let mut colors_found = vec![];
            for p in bit_planes {
                println!("- Finding barcodes in bit plane...");
                let barcodes = recognize_grayscale_barcodes(&p);
                for b in barcodes {
                    // Attempt to parse this barcode.
                    let hay = String::from_utf8(b).unwrap();
                    //println!("Decoded barcode as {}", hay);
                    match re.captures(hay.as_str()) {
                        Some(cap) => {
                            // We've got a match, which means we have information on what we were able to read.
                            let dpi = cap.get(1).unwrap().as_str().parse::<u16>().unwrap();
                            let colors = cap.get(2).unwrap().as_str().parse::<u16>().unwrap();
                            let color_num = cap.get(3).unwrap().as_str().parse::<u16>().unwrap();
                            found_barcodes.push((dpi, colors, color_num));
                            dpis_found.push(dpi);
                            colors_found.push(colors);
                        },
                        None => { }
                    }
                }
            }

            // Check to see if the maximum we found was fully parsed.
            dpis_found.sort();
            dpis_found.dedup();
            colors_found.sort();
            colors_found.dedup();
            if dpis_found.len() > 0 && colors_found.len() > 0 {
                println!("- Highest DPI found: {}", dpis_found[dpis_found.len() - 1]);
                println!("- Highest colors found: {}", colors_found[colors_found.len() - 1]);

                // Print the header.
                println!();
                println!("Found at this level:");
                print!("     ");
                for c in colors_found.clone() {
                    print!("{}", format!("{:^5}", c));
                }
                println!();

                // Print each row
                for d in dpis_found {
                    print!("{}", format!("{:<5}", d));
                    for c in colors_found.clone() {
                        // Check if we have barcodes for each bit plane.
                        let mut found_all_colors = true;
                        let mut found_some_colors = false;
                        let num_planes = u16::ilog2(c);
                        //println!("Number of colors to search for barcodes {}", num_colors);
                        for e in 1..(num_planes + 1) as u16 {
                            let mut found_color = false;
                            let barcodes_to_search = found_barcodes.clone();
                            for (dpi, colors, color_num) in barcodes_to_search {
                                if dpi == d && colors == c && color_num == e {
                                    found_color = true;
                                    //println!("Found barcode for {}, {}, {}", dpi, colors, color_num);
                                    break;
                                }
                            }
                            if found_color {
                                found_some_colors = true;
                            }
                            else {
                                //println!("Didn't find colors for {}, {}, {}", d, c, e);
                                found_all_colors = false;
                            }
                        }
                        if found_all_colors {
                            print!("  *  ");
                        }
                        else if found_some_colors {
                            located_all = false;
                            print!("  ?  ");
                        }
                        else {
                            located_all = false;
                            print!("     ");
                        }
                    }
                    println!();
                }
            }
            else {
                println!("- Did not find any usable barcodes at this color depth");
            }
            if located_all {
                println!();
                println!("Success!  All levels successfully found!  Stopping search.");
                break;
            }
            else {
                println!();
                println!("Did not find complete combinations of DPI and colors.  Trying again at lower color depth...");
                println!();
                println!("=====");
                println!();
            }
        }
    }
}