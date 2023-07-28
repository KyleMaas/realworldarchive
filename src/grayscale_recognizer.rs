use image::{DynamicImage, GenericImageView};
use image::imageops::FilterType;
//use rqrr::PreparedImage;
use bardecoder;
use image::imageops;
use image::Rgba;
use imageproc::rect::Rect;
use imageproc::drawing::*;

pub fn recognize_grayscale_barcodes(in_image: &DynamicImage) -> Vec<Vec<u8>> {
    // Need GenericImageView trait to be able to use width() and height().
    let sized_up = in_image.resize(in_image.width() * 2, in_image.height() * 2, FilterType::Nearest);
    let mut recognized_fragments = vec![];

    let decoder = bardecoder::default_decoder();

    // Decode in blocks of this size.
    let decode_block_size = 800;
    let mut x = 0;
    let mut y = 0;
    let quiet_zone = 200;

    println!("Attempting to decode");
    while y + decode_block_size / 2 < sized_up.height() {
        println!("Decoding block at {}, {}", x, y);
        let mut w = decode_block_size;
        if x + decode_block_size > sized_up.width() {
            w = sized_up.width() - x;
        }
        let mut h = decode_block_size;
        if y + decode_block_size > sized_up.height() {
            h = sized_up.height() - y;
        }
        let image_chunk = sized_up.view(x, y, w, h);

        // Add a large white border around the chunk to work around https://github.com/piderman314/bardecoder/issues/50
        /*let mut new_image = DynamicImage::new_rgb8(decode_block_size + quiet_zone * 2, decode_block_size + quiet_zone * 2);
        draw_filled_rect_mut(&mut new_image, Rect::at(0, 0).of_size(decode_block_size + quiet_zone * 2, decode_block_size + quiet_zone * 2), Rgba([255, 255, 255, 0]));
        imageops::overlay(&mut new_image, &image_chunk.to_image(), quiet_zone as i64, quiet_zone as i64);

        let results = decoder.decode(&new_image); //&(DynamicImage::ImageRgba8(image_chunk.to_image())));*/

        // Once https://github.com/piderman314/bardecoder/issues/50 is resolved, the lines above can be replaced with:
        let results = decoder.decode(&*image_chunk);
        
        println!("Done decoding - found {} results", results.len());
        for result in results {
            match result {
                Result::Ok(r) => recognized_fragments.push(r.as_bytes().to_vec()),
                Result::Err(e) => println!("Error in QR code results: {}", e),
                //Result::Err(_e) => { /* Ignore errors, because there will be a lot of them */} //println!("Error in QR code results: {}", e),
            }
        }
        x += decode_block_size / 2;
        if x + decode_block_size / 2 > sized_up.width() {
            x = 0;
            y += decode_block_size / 2;
        }
    }
    recognized_fragments
}