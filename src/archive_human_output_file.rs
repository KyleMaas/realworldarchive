// SPDX-License-Identifier: MIT OR Apache-2.0+ OR Zlib

use image::{RgbImage, Rgb};
use image::imageops;
use hsl::HSL;
use imageproc::rect::Rect;
use imageproc::drawing::*;
use rusttype::{Scale, Font};

#[derive(Copy, Clone)]
pub enum OutputFormat {
    PNG
}

#[derive(Copy, Clone)]
pub struct OutputMargins {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32
}

pub struct ArchiveHumanOutputFile<'a> {
    document_header: &'a str,
    document_footer: &'a str,
    total_pages: u16,
    out_file: &'a str,
    format: OutputFormat,
    width: f32,
    height: f32,
    text_height: f32,
    dpi: u16,
    margins: OutputMargins,
    colors: Vec<Rgb<u8>>
}

impl<'a> ArchiveHumanOutputFile<'a> {
    pub fn new(out_file: &'a str, format: OutputFormat) -> ArchiveHumanOutputFile<'a> {
        //let increment_per_color = 320.0 / 6.0; // We're going to exclude the far end of the range, since it doesn't print well and turns out as red.
        let colors_hsl = [
            HSL{h: 0.0, s: 0.0, l:0.0}, // Black
            HSL{h: 0.0, s: 1.0, l:0.5}, // Red
            HSL{h: 35.0, s: 1.0, l:0.5}, // Orange
            HSL{h: 60.0, s: 1.0, l:0.5}, // Yellow
            HSL{h: 120.0, s: 1.0, l:0.5}, // Green
            HSL{h: 200.0, s: 1.0, l:0.5}, // Blue
            HSL{h: 270.0, s: 1.0, l:0.6}, // Violet
            HSL{h: 0.0, s: 0.0, l:1.0} // White
        ];
        ArchiveHumanOutputFile {
            document_header: "Real World Archive",
            document_footer: "Page {{page_num}}",
            total_pages: 1,
            out_file: out_file,
            format: format,
            width: 8.5,
            height: 11.0,
            text_height: 0.25,
            dpi: 600,
            margins: OutputMargins {
                top: 0.25,
                right: 0.25,
                bottom: 0.5,
                left: 0.25
            },
            colors: colors_hsl.iter().map(|h| { let c = h.to_rgb(); Rgb([c.0, c.1, c.2]) }).collect()
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn dpi(mut self, dpi: u16) -> Self {
        self.dpi = dpi;
        self
    }

    pub fn colors(mut self, colors: &Vec<Rgb<u8>>) -> Self {
        self.colors = colors.to_vec();
        self
    }

    pub fn document_header(mut self, header: &'a str) -> Self {
        self.document_header = header;
        self
    }

    pub fn document_footer(mut self, footer: &'a str) -> Self {
        self.document_footer = footer;
        self
    }

    pub fn set_document_footer(&mut self, footer: &'a str) {
        self.document_footer = footer;
    }

    pub fn total_pages(mut self, pages: u16) -> Self {
        self.total_pages = pages;
        self
    }

    pub fn set_total_pages(&mut self, pages: u16) {
        self.total_pages = pages;
    }

    pub fn finalize(self) -> ArchiveHumanOutputFile<'a> {
        ArchiveHumanOutputFile {
            document_header: self.document_header,
            document_footer: self.document_footer,
            total_pages: self.total_pages,
            out_file: self.out_file,
            format: self.format,
            width: self.width,
            height: self.height,
            text_height: self.text_height,
            dpi: self.dpi,
            margins: self.margins,
            colors: self.colors
        }
    }

    pub fn get_barcode_image_size(&self) -> (u32, u32) {
        let width_units = self.width - self.margins.left - self.margins.right;
        let height_units = self.height - self.margins.top - self.margins.bottom - (self.text_height * 2.0);
        let dpi_float = self.dpi as f32;
        let width_pixels = (width_units * dpi_float).round() as u32;
        let height_pixels = (height_units * dpi_float).round() as u32;
        (width_pixels, height_pixels)
    }

    pub fn get_dpi(&self) -> u16 {
        self.dpi
    }

    /*pub fn get_colors(&self) -> &Vec<Rgb<u8>> {
        &self.colors
    }*/

    pub fn write_page(&self, code_image: &RgbImage, page_num: u16) {
        // Format the barcode image into the bounds on the page where it should be, and add metadata.
        // Build a blank full page.
        let dpi_float = self.dpi as f32;
        let page_width_pixels = (self.width * dpi_float).round() as u32;
        let page_height_pixels = (self.height * dpi_float).round() as u32;
        let mut out_image = RgbImage::new(page_width_pixels, page_height_pixels);
        draw_filled_rect_mut(&mut out_image, Rect::at(0, 0).of_size(page_width_pixels, page_height_pixels), Rgb([255, 255, 255]));

        // Copy the barcode to within the margins.
        imageops::overlay(&mut out_image, code_image, (self.margins.left * dpi_float) as i64, ((self.margins.top + self.text_height) * dpi_float) as i64);

        let num_colors = self.colors.len();
        //let num_bits_colors = (num_colors as f64).log(2.0) as u8;

        // Add the header.
        let font_data: &[u8] = include_bytes!("Seshat-Regular.ttf");
        let font = Font::try_from_bytes(font_data).unwrap();
        let header_substituted = self.document_header
            .replace("{{page_num}}", &(page_num.to_string()))
            .replace("{{total_pages}}", &(self.total_pages.to_string()))
            .replace("{{dpi}}", &(self.dpi.to_string()))
            .replace("{{total_overlay_colors}}", &(num_colors.to_string()));
        draw_text_mut(&mut out_image, Rgb([0, 0, 0]), (self.margins.left * dpi_float) as i32, (self.margins.top * dpi_float) as i32, Scale::uniform(self.text_height * dpi_float), &font, &header_substituted);

        // Add the footer.
        let footer_substituted = self.document_footer
            .replace("{{page_num}}", &(page_num.to_string()))
            .replace("{{total_pages}}", &(self.total_pages.to_string()))
            .replace("{{dpi}}", &(self.dpi.to_string()))
            .replace("{{total_overlay_colors}}", &(num_colors.to_string()));
        let footer_top = page_height_pixels - ((self.margins.bottom + self.text_height) * dpi_float) as u32;
        draw_text_mut(&mut out_image, Rgb([0, 0, 0]), (self.margins.left * dpi_float) as i32, footer_top as i32, Scale::uniform(self.text_height * dpi_float), &font, &footer_substituted);

        // Add the color palette, but only if we're actually using colors.
        if self.colors.len() > 2 {
            let max_palette_width = (page_width_pixels - ((self.margins.left * dpi_float) as u32) - ((self.margins.right * dpi_float) as u32)) / 2;
            let colors_except_bw = self.colors.len() as u32 - 2;
            let mut rows = 0;
            let mut colors_per_row;
            let mut palette_top;
            let mut palette_height;
            let mut swatch_size;
            let mut palette_width;
            let palette_left;
            let palette_border;
            loop {
                rows = rows + 1;
                colors_per_row = (colors_except_bw + rows - 1) / rows; // colors_except_bw / rows, rounding up
                palette_top = footer_top;
                palette_height = (self.text_height * dpi_float) as u32;
                swatch_size = palette_height / rows;
                palette_width = colors_per_row * swatch_size;
                //println!("Rows: {}", rows);
                //println!("Colors per row: {}", colors_per_row);
                //println!("Page width: {}", page_width_pixels);
                //println!("Palette width: {}", palette_width);
                // Exit the loop if we've packed it correctly.
                if palette_width <= max_palette_width {
                    // Only calculate these if we're reasonably sure we won't overflow.
                    palette_left = page_width_pixels - ((self.margins.right * dpi_float) as u32) - palette_width;
                    palette_border = swatch_size / 4;
                    break;
                }
            }
            draw_filled_rect_mut(&mut out_image, Rect::at(palette_left as i32, palette_top as i32).of_size(palette_width, palette_height), Rgb([0, 0, 0]));
            for c in 0..(self.colors.len() - 2) {
                let x = (palette_left + ((c as u32 % colors_per_row) * swatch_size) + palette_border) as i32;
                let y = (palette_top + (c as u32 / colors_per_row) * swatch_size + palette_border) as i32;
                draw_filled_rect_mut(&mut out_image, Rect::at(x, y).of_size(swatch_size - palette_border * 2, swatch_size - palette_border * 2), self.colors[c + 1]);
            }
        }

        // Save it out.
        let numbered_filename = format!("{}{}.png", self.out_file, page_num);
        println!("Writing to {}", numbered_filename);
        out_image.save(numbered_filename).unwrap();
    }
}