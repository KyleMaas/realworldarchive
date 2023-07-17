use crate::archive_human_output_file::*;
use crate::archive_human_input_file::*;
use crate::grayscale_recognizer::recognize_grayscale_barcodes;
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

    fn multiplex_barcodes_into_image(barcodes: Vec<RgbImage>, colors: &Vec<Rgb<u8>>) -> RgbImage {
        let width = barcodes[0].dimensions().0;
        let height = barcodes[0].dimensions().1;
        let gray_code_to_decimal = [
            0,
            1,
            3,
            2,
            6,
            7,
            5,
            4,
            12,
            13,
            15,
            14,
            10,
            11,
            9,
            8
        ];
        let mut out_image = RgbImage::new(width, height);
        for x in 0..width {
            for y in 0..height {
                let mut bits_pixel_value = 0;
                for c in 0..barcodes.len() {
                    let bit_pixel = barcodes[c].get_pixel(x, y);
                    bits_pixel_value |= if bit_pixel[0] > 127 { 1 << c } else { 0 };
                }

                // Treat the bits as a Gray code and translate to color index equivalent.
                // This way, if the hue recognizer is off by one, we only lose one bit instead of possibly several.
                // We may also be able to use this to our advantage, in that the "111" value is not the background color, so we can indirectly determine how many distinct hues we have.
                let decimal_output = gray_code_to_decimal[bits_pixel_value];

                out_image.put_pixel(x, y, colors[decimal_output]);
            }
        }
        out_image
    }

    pub fn encode(&self, writer: &ArchiveHumanOutputFile) {
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
        let ec_level = EcLevel::H;
        for y in 0..4 {
            // Fill up the size of QR we're generating.
            let x = 0;
            let color = "B&W";
            let qrcode_version = Version::Normal(largest_barcode_version >> y);
            let dpi = full_dpi >> y;
            println!("Generating B&W barcode at {} DPI", dpi);
            let message = format!("Test at {} DPI in color {} ===== ", dpi, color);
            let bits = StressTestPage::generate_barcode_filling_bits(qrcode_version, ec_level, &message);

            // Generate the QR code.
            let code = QrCode::with_bits(bits, ec_level).unwrap();
            let code_image = code.render::<Rgb<u8>>().module_dimensions(1 << y, 1 << y).quiet_zone(false).build();
            imageops::overlay(&mut out_image, &code_image, x, (((y * large_barcode_height) as u32) + quiet_zone) as i64);
        }

        // Now do one for color.
        let colors = writer.get_colors();
        let num_bits_colors = (colors.len() as f64).log(2.0) as u8;
        println!("We can generate {} barcodes for {} colors", num_bits_colors, colors.len());
        for y in 0..4 {
            let x = barcode_image_size.0 / 2;
            let color_barcodes:Vec<RgbImage> = (0..num_bits_colors).map(|c:u8| {
                // Fill up the size of QR we're generating.
                let color = format!("Color #{}", c);
                let qrcode_version = Version::Normal(largest_barcode_version >> y);
                let dpi = full_dpi >> y;
                println!("Generating color #{} barcode at {} DPI", c, dpi);
                // Stick some spaces on the front so we're not repeating the same starting data in the same positions across barcodes.
                let message = format!("{}Test at {} DPI in color {} ===== ", (" ").repeat(c as usize), dpi, color);
                let bits = StressTestPage::generate_barcode_filling_bits(qrcode_version, ec_level, &message);

                // Generate the QR code.
                let code = QrCode::with_bits(bits, ec_level).unwrap();
                code.render::<Rgb<u8>>().module_dimensions(1 << y, 1 << y).quiet_zone(false).build()
            }).collect::<Vec<RgbImage>>();
            let code_image = StressTestPage::multiplex_barcodes_into_image(color_barcodes, &colors);
            imageops::overlay(&mut out_image, &code_image, x as i64, (((y * large_barcode_height) as u32) + quiet_zone) as i64);
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