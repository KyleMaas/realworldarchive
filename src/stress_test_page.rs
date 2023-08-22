// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use crate::archive_human_output_file::*;
use crate::archive_human_input_file::*;
use crate::grayscale_recognizer::recognize_grayscale_barcodes;
use crate::color_multiplexer::ColorMultiplexer;
extern crate image;
use image::{RgbImage, Rgb};
use image::imageops;
use imageproc::rect::Rect;
use imageproc::drawing::*;
use qrencode::QrCode;
use qrencode::bits::Bits;
use qrencode::types::{Version, EcLevel, Mode};

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
            for y in 0..4 {
                let num_colors = (2 as u8).pow(num_colors_bits as u32);
                let multiplexer = ColorMultiplexer::new(num_colors);

                // Fill up the size of QR we're generating.
                let color_description = if num_colors_bits < 2 {
                    String::from("B&W")
                }
                else {
                    format!("{} Colors", num_colors)
                };
                let color_barcodes:Vec<RgbImage> = (0..num_colors_bits).map(|c:u8| {
                    let qrcode_version = Version::Normal(largest_barcode_version >> y);
                    let dpi = full_dpi >> y;
                    println!("Generating {} barcode at {} DPI", color_description, dpi);
                    let space_for_color = String::from("=").repeat(c as usize);
                    let message = format!("{} Test at {} DPI in color {} ===== ", space_for_color, dpi, color_description);
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

    pub fn decode(&self, reader: &ArchiveHumanInputFile) {
        println!("Reading image");
        let image = reader.read_page().unwrap();

        // For now, convert to B&W.
        let bw = image; //.into_luma();
        let barcodes = recognize_grayscale_barcodes(&bw);
        for b in barcodes {
            println!("{}", String::from_utf8(b).unwrap());
        }
    }
}