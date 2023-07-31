// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use image::Rgb;
use image::Rgba;
use image::RgbImage;
use image::DynamicImage;
use image::GenericImage;
use image::GenericImageView;
use gray_codes::GrayCode8;
use hsl::HSL;
use palette::cast;
use palette::{white_point::D65, FromColor, IntoColor, Lab, Srgb};
use kmeans_colors::{get_kmeans_hamerly, Calculate, Kmeans, MapColor, Sort};
use std::collections::HashSet;

pub struct ColorMultiplexer {
    colors_rgb: Vec<Rgb<u8>>,
    colors_hsl: Vec<HSL>
}

fn reorder_by_gray_code(num_colors: u8, colors_rgb: Vec<Rgb<u8>>, colors_hsl: Vec<HSL>) -> (Vec<Rgb<u8>>, Vec<HSL>) {
    // We're now assuming that the last color in the list is white.
    // Put them in Gray code order.
    println!("Number of colors: {}", num_colors);
    let gray_codes = GrayCode8::with_bits(num_colors.ilog2() as usize).collect::<Vec<u8>>();
    let mut gray_code_order_rgb: Vec<Rgb<u8>> = Vec::with_capacity(num_colors as usize);
    let mut gray_code_order_hsl: Vec<HSL> = Vec::with_capacity(num_colors as usize);
    for _i in 0..num_colors {
        gray_code_order_rgb.push(Rgb([0, 0, 0]));
        gray_code_order_hsl.push(HSL { h: 0.0, s: 0.0, l: 0.0 });
    }
    let mut white_index = num_colors as usize - 1;
    for (i, g) in gray_codes.iter().enumerate() {
        let gray_code = *g as usize;
        if gray_code >= colors_rgb.len() - 1 {
            white_index = i;
        }
        gray_code_order_rgb[i] = colors_rgb[gray_code];
        gray_code_order_hsl[i] = colors_hsl[gray_code];
    }

    // Remove white from its original order and place it at the end.
    // This may not be true white, but it's the whitest color we have from the incoming palette.
    let white_rgb = gray_code_order_rgb.remove(white_index);
    let white_hsl = gray_code_order_hsl.remove(white_index);
    gray_code_order_rgb.push(white_rgb);
    gray_code_order_hsl.push(white_hsl);

    return (gray_code_order_rgb, gray_code_order_hsl);
}

fn generate_palette(num_colors_unrounded: u8) -> (Vec<Rgb<u8>>, Vec<HSL>) {
    if num_colors_unrounded == 2 {
        return (vec![Rgb([0, 0, 0]), Rgb([255, 255, 255])], vec![HSL { h: 0.0, s: 0.0, l: 0.0 }, HSL { h: 0.0, s: 0.0, l: 1.0 }]);
    }
    println!("Number of colors: {}", num_colors_unrounded);
    let num_colors = (2 as u8).pow(num_colors_unrounded.ilog2());
    println!("Number of colors rounded: {}", num_colors);
    let mut colors_rgb = vec![Rgb([0, 0, 0])];
    let mut colors_hsl = vec![HSL { h: 0.0, s: 0.0, l: 0.0 }];
    for c in 0..(num_colors - 2) {
        let angle = (c as f64) / ((num_colors - 2) as f64) * 360.0;
        let mut l = 0.5;
        if num_colors > 8 {
            // Alternate between darker and lighter colors to make repalettizing them later easier.
            let base = 0.6;
            let offset = 0.1;
            l = base + if (c % 2) == 0 { offset } else { -offset };
        }
        let hsl = HSL { h: angle, s: 1.0, l: l };
        let (r, g, b) = hsl.to_rgb();
        colors_rgb.push(Rgb([r, g, b]));
        colors_hsl.push(hsl);
    }

    // Add white at the end.
    colors_rgb.push(Rgb([255, 255, 255]));
    colors_hsl.push(HSL { h: 0.0, s: 0.0, l: 1.0 });

    reorder_by_gray_code(num_colors, colors_rgb, colors_hsl)
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

    pub fn num_colors(&self) -> u8 {
        self.colors_rgb.len() as u8
    }

    pub fn get_rgb(&self) -> &Vec<Rgb<u8>> {
        &self.colors_rgb
    }

    pub fn palettize_from_image(&mut self, img: &DynamicImage) {
        println!("Repalettizing from {:?}", self.colors_rgb);

        let num_colors = self.num_colors();
        if num_colors <= 2 {
            // It's monochrome - there's no need to repalettize.
            return;
        }

        // Find the most dominant colors in the image.
        // We only really want the palette sample in the bottom right corner, ignoring the antialiased lettering.
        let image_chunk = img.view(img.width() / 2, img.height() / 8 * 7, img.width() / 2, img.height() / 8).to_image();
        //image_chunk.save("test_out/palettediagnostic.png").unwrap();
        /*let img_vec = image_chunk.as_raw();
        let pixels: Vec<Srgb> = cast::from_component_slice::<Srgb<u8>>(img_vec)
            .iter()
            .map(|x| x.into_format::<f32>().into_color())
            .collect();*/
        // Have to do this differently than the reference code because of differences in color byte order (GBR or something like that instead of RGB) messing things up massively.
        let pixels: Vec<Srgb> = image_chunk.pixels()
            .map(|x| Srgb::new(x[0] as f32 / 255.0, x[1] as f32 / 255.0, x[2] as f32 / 255.0))
            .collect();
        /*for i in 0..pixels.len() {
            if pixels[i].red > 0.5 && pixels[i].red < 0.9 {
                println!("Pixel: {}, {}, {}", (pixels[i].red * 255.0) as u8, (pixels[i].green * 255.0) as u8, (pixels[i].blue * 255.0) as u8);
                break;
            }
        }*/
        //println!("Pixel colors: {:?}", pixels);
        /*let mut pixels: Vec<Srgb> = vec![];
        for i in (0..img_vec.len()).step_by(3) {
            pixels.push(Srgb::new(img_vec[i] as f32 / 255.0, img_vec[i + 1] as f32 / 255.0, img_vec[i + 2] as f32 / 255.0));
        }*/
        let mut result = Kmeans::new();
        for i in 0..4 {
            let run_result = get_kmeans_hamerly(
                num_colors as usize,
                8, // Default for the CLI version, per https://github.com/okaneco/kmeans-colors/issues/50#issuecomment-1073156666
                0.0025, // Recommended in https://github.com/okaneco/kmeans-colors/issues/50#issuecomment-1073156666
                false,
                &pixels,
                6972593 + i as u64
            );
            if run_result.score < result.score {
                result = run_result;
            }
        }

        // Convert colors to Srgb and then to HSL.
        let mut colors_rgb: Vec<Rgb<u8>> = vec![];
        let mut colors_hsl: Vec<HSL> = vec![];
        //println!("Original colors: {:?}", result.centroids);
        for c in result.centroids {
            let r = (c.red * 255.0) as u8;
            let g = (c.green * 255.0) as u8;
            let b = (c.blue * 255.0) as u8;
            colors_rgb.push(Rgb([r, g, b]));
            colors_hsl.push(HSL::from_rgb(&[r, g, b]));
        }
        //println!("Remapped colors: {:?}", colors_rgb);

        // Sort the colors by HSL hue, putting the darkest color (which we'll treat as black) at the start and the lightest color (which we'll treat as white) at the end.
        // Bubble sort for simplicity.
        for i in 0..colors_hsl.len() - 1 {
            for j in (i + 1)..colors_hsl.len() {
                if colors_hsl[j].h < colors_hsl[i].h {
                    let temp_hsl = colors_hsl[i];
                    colors_hsl[i] = colors_hsl[j];
                    colors_hsl[j] = temp_hsl;

                    let temp_rgb = colors_rgb[i];
                    colors_rgb[i] = colors_rgb[j];
                    colors_rgb[j] = temp_rgb;
                }
            }
        }
        // Search for black.
        let mut darkest_color_index = 0;
        for c in 0..colors_hsl.len() {
            if colors_hsl[c].l < colors_hsl[darkest_color_index].l {
                darkest_color_index = c;
            }
        }
        let dark_hsl = colors_hsl.remove(darkest_color_index);
        let dark_rgb = colors_rgb.remove(darkest_color_index);
        colors_hsl.insert(0, dark_hsl);
        colors_rgb.insert(0, dark_rgb);
        //println!("Colors after reordering black {:?}", colors_rgb);

        // Search for white.
        let mut lightest_color_index = colors_hsl.len() - 1;
        for c in 0..colors_hsl.len() {
            if colors_hsl[c].l > colors_hsl[lightest_color_index].l {
                lightest_color_index = c;
            }
        }
        let light_hsl = colors_hsl.remove(lightest_color_index);
        let light_rgb = colors_rgb.remove(lightest_color_index);
        colors_hsl.push(light_hsl);
        colors_rgb.push(light_rgb);
        //println!("Colors after reordering white {:?}", colors_rgb);

        // Reorder by Gray code.
        (self.colors_rgb, self.colors_hsl) = reorder_by_gray_code(num_colors, colors_rgb, colors_hsl);

        println!("Repalettized to {:?}", self.colors_rgb);

        //panic!("Stopping for now");
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