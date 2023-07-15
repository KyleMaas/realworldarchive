use image::Rgb;
use image::RgbImage;
use gray_codes::GrayCode8;
use hsl::HSL;

pub struct ColorMultiplexer {
    colors: Vec<Rgb<u8>>
}

fn generate_palette(num_colors_unrounded: u8) -> Vec<Rgb<u8>> {
    if num_colors_unrounded == 2 {
        return vec![Rgb([0, 0, 0]), Rgb([255, 255, 255])];
    }
    println!("Number of colors: {}", num_colors_unrounded);
    let num_colors = (2 as u8).pow(num_colors_unrounded.ilog2());
    println!("Number of colors rounded: {}", num_colors);
    let gray_codes = GrayCode8::with_bits(num_colors.ilog2() as usize).collect::<Vec<u8>>();
    let mut colors = vec![Rgb([0, 0, 0])];
    for c in 0..(num_colors - 1) {
        let angle = (c as f64) / (num_colors as f64) * 360.0;
        let hsl = HSL { h: angle, s: 1.0, l: 0.5 };
        let (r, g, b) = hsl.to_rgb();
        colors.push(Rgb([r, g, b]))
    }

    // Put them in Gray code order.
    println!("Number of colors: {}", num_colors);
    let mut gray_code_order: Vec<Rgb<u8>> = Vec::with_capacity(num_colors as usize);
    for _i in 0..num_colors {
        gray_code_order.push(Rgb([0, 0, 0]));
    }
    let mut white_index = num_colors as usize - 1;
    for (i, g) in gray_codes.iter().enumerate() {
        let gray_code = *g as usize;
        if gray_code >= colors.len() {
            white_index = i;
        }
        else {
            gray_code_order[i] = colors[gray_code];
        }
    }

    // Remove white from its original order and place it at the end.
    gray_code_order.remove(white_index);
    gray_code_order.push(Rgb([255, 255, 255]));

    return gray_code_order;
}

impl<'a> ColorMultiplexer {
    pub fn new(num_colors: u8) -> ColorMultiplexer {
        ColorMultiplexer {
            colors: generate_palette(num_colors)
        }
    }

    pub fn finalize(self) -> ColorMultiplexer {
        ColorMultiplexer {
            colors: self.colors
        }
    }

    pub fn num_planes(&self) -> u8 {
        self.colors.len().ilog2() as u8
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
                out_image.put_pixel(x, y, self.colors[out_palette_index]);
            }
        }
        out_image
    }
}