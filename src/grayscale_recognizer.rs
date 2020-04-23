use image::{DynamicImage, GenericImageView};
use image::imageops::FilterType;
//use rqrr::PreparedImage;
use bardecoder;

pub fn recognize_grayscale_barcodes(in_image: &DynamicImage) -> Vec<Vec<u8>> {
    // Need GenericImageView trait to be able to use width() and height().
    let sized_up = in_image; //in_image.resize(in_image.width() * 4, in_image.height() * 4, FilterType::Nearest);
    let mut recognized_fragments = vec![];

    let decoder = bardecoder::default_decoder();

    println!("Attempting to decode");
    let results = decoder.decode(&sized_up);
    println!("Done decoding");
    for result in results {
        match result {
            Result::Ok(r) => recognized_fragments.push(r.as_bytes().to_vec()),
            Result::Err(e) => println!("Error in QR code results: {}", e),
        }
    }
    recognized_fragments
}