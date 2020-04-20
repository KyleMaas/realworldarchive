use image::GrayImage;
use rqrr::PreparedImage;

pub fn recognize_grayscale_barcodes(in_image: &GrayImage) -> Vec<Vec<u8>> {
    let mut recognized_fragments = vec![];

    // Egad, this is a horrible way to do this, but I can't seem to get PreparedImage to take a GrayImage.
    let mut rqrr_img = PreparedImage::prepare_from_greyscale(in_image.width()as usize, in_image.height() as usize, |x:usize, y:usize| -> u8 { in_image.get_pixel(x as u32, y as u32).0[0] });

    println!("Image size: {} x {}", in_image.width(), in_image.height());

    let grids = rqrr_img.detect_grids();
    println!("{} Barcodes found", grids.len());

    for g in grids {
        let (_meta, content) = g.decode().unwrap();
        println!("Found barcode with content {}", content);
        recognized_fragments.push(content.as_bytes().to_vec());
    }

    recognized_fragments
}