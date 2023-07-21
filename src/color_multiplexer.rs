use image::Rgb;
use image::Rgba;
use image::RgbImage;
use image::DynamicImage;
use image::GenericImage;
use image::GenericImageView;
use gray_codes::GrayCode8;
use hsl::HSL;

pub struct ColorMultiplexer {
    colors_rgb: Vec<Rgb<u8>>,
    colors_hsl: Vec<HSL>
}

fn generate_palette(num_colors_unrounded: u8) -> (Vec<Rgb<u8>>, Vec<HSL>) {
    if num_colors_unrounded == 2 {
        return (vec![Rgb([0, 0, 0]), Rgb([255, 255, 255])], vec![HSL { h: 0.0, s: 0.0, l: 0.0 }, HSL { h: 0.0, s: 0.0, l: 1.0 }]);
    }
    println!("Number of colors: {}", num_colors_unrounded);
    let num_colors = (2 as u8).pow(num_colors_unrounded.ilog2());
    println!("Number of colors rounded: {}", num_colors);
    let gray_codes = GrayCode8::with_bits(num_colors.ilog2() as usize).collect::<Vec<u8>>();
    let mut colors_rgb = vec![Rgb([0, 0, 0])];
    let mut colors_hsl = vec![HSL { h: 0.0, s: 0.0, l: 0.0 }];
    for c in 0..(num_colors - 1) {
        let angle = (c as f64) / (num_colors as f64) * 360.0;
        let hsl = HSL { h: angle, s: 1.0, l: 0.5 };
        let (r, g, b) = hsl.to_rgb();
        colors_rgb.push(Rgb([r, g, b]));
        colors_hsl.push(hsl);
    }

    // Put them in Gray code order.
    println!("Number of colors: {}", num_colors);
    let mut gray_code_order_rgb: Vec<Rgb<u8>> = Vec::with_capacity(num_colors as usize);
    let mut gray_code_order_hsl: Vec<HSL> = Vec::with_capacity(num_colors as usize);
    for _i in 0..num_colors {
        gray_code_order_rgb.push(Rgb([0, 0, 0]));
        gray_code_order_hsl.push(HSL { h: 0.0, s: 0.0, l: 0.0 });
    }
    let mut white_index = num_colors as usize - 1;
    for (i, g) in gray_codes.iter().enumerate() {
        let gray_code = *g as usize;
        if gray_code >= colors_rgb.len() {
            white_index = i;
        }
        else {
            gray_code_order_rgb[i] = colors_rgb[gray_code];
            gray_code_order_hsl[i] = colors_hsl[gray_code];
        }
    }

    // Remove white from its original order and place it at the end.
    gray_code_order_rgb.remove(white_index);
    gray_code_order_hsl.remove(white_index);
    gray_code_order_rgb.push(Rgb([255, 255, 255]));
    gray_code_order_hsl.push(HSL { h: 0.0, s: 0.0, l: 1.0 });

    return (gray_code_order_rgb, gray_code_order_hsl);
}

impl<'a> ColorMultiplexer {
    pub fn new(num_colors: u8) -> ColorMultiplexer {
        let (rgb, hsl) = generate_palette(num_colors);
        ColorMultiplexer {
            colors_rgb: rgb,
            colors_hsl: hsl
        }
    }

    pub fn finalize(self) -> ColorMultiplexer {
        ColorMultiplexer {
            colors_rgb: self.colors_rgb,
            colors_hsl: self.colors_hsl
        }
    }

    pub fn num_planes(&self) -> u8 {
        self.colors_rgb.len().ilog2() as u8
    }

    pub fn multiplex_planes(&self, p: Vec<RgbImage>) -> RgbImage {
        if p.len() != self.num_planes() as usize {
            panic!("Wrong number of color planes to multiplex");
        }

        let w = p[0].width();
        let h = p[0].height();
        let mut out_image = RgbImage::new(w, h);
        for x in 0..w {
            for y in 0..h {
                // Multiplex the pixels into a single color.
                let mut out_palette_index: usize = 0;
                for c in 0..p.len() {
                    let pixel = p[c].get_pixel(x, y);
                    if pixel[0] > 127 {
                        out_palette_index |= 1 << c;
                    }
                }
                out_image.put_pixel(x, y, self.colors_rgb[out_palette_index]);
            }
        }
        out_image
    }

    pub fn demultiplex_image(&self, color_image: &DynamicImage) -> Vec<DynamicImage> {
        let num_images = self.num_planes() as usize;
        let mut planes = vec![];

        // Fill the output array so we can start decoding.
        for _i in 0..num_images {
            planes.push(DynamicImage::new_luma8(color_image.width(), color_image.height()));
        }

        // Loop through each pixel and palettize it.
        for x in 0..color_image.width() {
            for y in 0..color_image.height() {
                let palette_index;
                let pixel = color_image.get_pixel(x, y);
                let hsl = HSL::from_rgb(&[pixel[0], pixel[1], pixel[2]]);
                if hsl.s < 0.5 {
                    // Special case - either black or white.
                    if hsl.l < 0.5 {
                        // Black
                        palette_index = 0;
                    }
                    else {
                        // White - always the last color in the palette
                        palette_index = self.colors_rgb.len() - 1;
                    }
                }
                else {
                    // Find the closest color
                    let mut closest_index = 0;
                    let mut best_hue_distance = 360.0;
                    for c in 1..self.colors_hsl.len() - 1 {
                        let this_color_distance = f64::abs(self.colors_hsl[c].h - hsl.h);
                        if this_color_distance < best_hue_distance {
                            closest_index = c;
                            best_hue_distance = this_color_distance;
                        }
                    }
                    palette_index = closest_index;
                }

                // We should have a palette index we can work with now.
                // Decode it into bits.
                //println!("Found palette index {}", palette_index);
                for p in 0..num_images {
                    let plane_bit_is_set = ((palette_index as u8) >> p) & 0x1;
                    if plane_bit_is_set != 0 {
                        planes[p].put_pixel(x, y, Rgba([255, 255, 255, 0]));
                    }
                    else {
                        planes[p].put_pixel(x, y, Rgba([0, 0, 0, 0]));
                    }
                }
            }
        }

        println!("Demultiplexed {} color planes", num_images);

        planes
    }
}