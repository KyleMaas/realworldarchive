use image::Rgb;
use image::imageops;
use qrcode::QrCode;
use qrcode::bits::Bits;
use qrcode::types::{Version, EcLevel, Mode};
use image::RgbImage;
use imageproc::rect::Rect;
use imageproc::drawing::*;

// Quiet zone size between QR codes, in pixels.  Default is a little more than the required 4, but not 10 like some folks recommend.  If this is unreliable, we might need to change it.
const QUIET_ZONE_SIZE:u8 = 6;

#[derive(Copy, Clone)]
pub enum BarcodeFormat {
    QR
}

pub type DamageLikelihoodMap = Box<dyn Fn(f32, f32) -> f32>;

// Always returns a constant damage likelihood.
pub fn make_constant_damage_map(l: f32) -> DamageLikelihoodMap {
    return Box::new(move |_x: f32, _y: f32| l);
}

// Returns the specified minimum in the center, progressing to the specified maximum as it gets to the edges of the page and sloping further toward the corners.
pub fn make_radial_damage_map(min: f32, max: f32) -> DamageLikelihoodMap {
    let diff = max - min;

    return Box::new(move |x: f32, y: f32| {
        let dist_from_center_x = (0.5 - x).abs();
        let dist_from_center_y = (0.5 - y).abs();
        return (min + (dist_from_center_x * dist_from_center_x + dist_from_center_y * dist_from_center_y).sqrt() * 2.0 * diff).min(1.0);
    });
}

struct BarcodeInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    damage_likelihood: f32,
    version: Version,
    ec_level: EcLevel,
    mode: Mode,
    capacity: u32
}

pub struct PageBarcodePacker {
    width: u32,
    height: u32,
    barcode_format: BarcodeFormat,
    colors: Vec<Rgb<u8>>,
    damage_likelihood_map: DamageLikelihoodMap,
    format_version: u8,
    packing_cached: bool,
    cache_barcodes: Vec<BarcodeInfo>,
    cache_bytes_per_page: u32
}

impl<'a> PageBarcodePacker {
    pub fn new(width: u32, height: u32, barcode_format: BarcodeFormat) -> PageBarcodePacker {
        let mut out = PageBarcodePacker {
            width,
            height,
            barcode_format,
            format_version: 1,
            colors: vec!(Rgb([0, 0, 0]), Rgb([255, 255, 255])),
            packing_cached: false,
            cache_barcodes: vec!(),
            damage_likelihood_map: make_constant_damage_map(0.5),
            cache_bytes_per_page: 0
        };
        out.ensure_barcodes_are_packed();
        out
    }

    pub fn barcode_format(mut self, f: BarcodeFormat) -> Self {
        self.packing_cached = false;
        self.barcode_format = f;
        self
    }

    pub fn damage_likelihood_map(mut self, m: DamageLikelihoodMap) -> Self {
        self.packing_cached = false;
        self.damage_likelihood_map = m;
        self
    }

    pub fn format_version(mut self, v: u8) -> Self{
        if self.format_version != v {
            self.packing_cached = false;
        }
        self.format_version = v;
        self
    }

    pub fn finalize(self) -> PageBarcodePacker {
        let mut out = PageBarcodePacker {
            width: self.width,
            height: self.height,
            barcode_format: self.barcode_format,
            colors: self.colors,
            damage_likelihood_map: self.damage_likelihood_map,
            format_version: self.format_version,
            packing_cached: self.packing_cached,
            cache_barcodes: self.cache_barcodes,
            cache_bytes_per_page: self.cache_bytes_per_page
        };
        out.ensure_barcodes_are_packed();
        out
    }

    fn ensure_barcodes_are_packed(&mut self) {
        if self.packing_cached {
            return;
        }
        // Figure out the barcode packing
        // For now, we're going to use the super-naiive method of packing full-sized barcodes and figuring out how much each can hold.
        // TODO: Figure out better packing of barcodes.
        // TODO: Randomize the order of the barcodes on the page.
        let mut next_x: u32 = 0;
        let mut next_y: u32 = 0;
        let v = 40; // Size ("version") of QR code
        let qrv = Version::Normal(v);
        let barcode_size: u32 = qrv.width() as u32;
        self.cache_bytes_per_page = 0;
        while next_y < self.height {
            let dl = (self.damage_likelihood_map)((next_x + barcode_size / 2) as f32 / self.width as f32, (next_y + barcode_size / 2) as f32 / self.height as f32);
            let ec = 
                if dl >= 0.0 && dl < 0.25 {
                    EcLevel::L
                }
                else if dl >= 0.25 && dl < 0.5 {
                    EcLevel::M
                }
                else if dl >= 0.5 && dl < 0.75 {
                    EcLevel::Q
                }
                else {
                    EcLevel::H
                };
            let bits = Bits::new(qrv);
            let max_bits = bits.max_len(ec).unwrap();
            let metadata_bits = Mode::Byte.length_bits_count(qrv) + 4 + qrv.mode_bits_count();
            let max_bytes: u32 = (max_bits - metadata_bits) as u32 / 8;
            let bytes_for_version = 1;
            let bytes_for_page_number = 2;
            let bytes_for_barcode_number = 2;
            let bytes_for_offset = 6;
            let bytes_for_total_length = 6;
            let bytes_for_hash = 3;
            let data_capacity_per_color_bit: u32 = max_bytes - bytes_for_version - bytes_for_page_number - bytes_for_barcode_number - bytes_for_offset - bytes_for_total_length - bytes_for_hash;
            let data_capacity = data_capacity_per_color_bit * self.colors.len().ilog2();

            let new_code = BarcodeInfo {
                x: next_x,
                y: next_y,
                width: barcode_size,
                height: barcode_size,
                damage_likelihood: dl,
                version: qrv,
                ec_level: ec,
                mode: Mode::Byte,
                capacity: data_capacity
            };
            self.cache_barcodes.push(new_code);
            self.cache_bytes_per_page += data_capacity;

            // Move to the next one.
            next_x += barcode_size + QUIET_ZONE_SIZE as u32;
            if next_x + barcode_size > self.width {
                next_x = 0;
                next_y += barcode_size + QUIET_ZONE_SIZE as u32;
            }
        }
        self.packing_cached = true;
    }

    pub fn repack_barcodes_for_page_length(&mut self, min_needed_length: u32) -> bool {
        // TODO: Implement this function such that we can try to expand the size of barcodes by integer multiples of the pixel size so we can use larger, lower-DPI barcodes for better readability.
        // Return code indicates if we were able to successfully fit the required quantity of data onto the page at a different resolution.
        if min_needed_length > self.cache_bytes_per_page {
            panic!("We can only try to shrink the capacity of pages, not grow them.")
        }
        else if min_needed_length == self.cache_bytes_per_page {
            // We're already optimized - no sense in trying anything less.
            // Let the caller know that we've expanded the barcodes to the largest size we can.
            return false;
        }
        // TODO: Actual implementation here.
        // Always returning false here until we can actually implement the optimization system.
        return false;
    }

    pub fn data_bytes_per_page(&self) -> u32 {
        self.cache_bytes_per_page
    }

    fn generate_barcode_filling_bits(&self, qrcode_version: Version, ec_level: EcLevel, byte_array: &[u8]) -> Bits {
        let mut bits = Bits::new(qrcode_version);
        bits.push_byte_data(byte_array).unwrap();
        bits.push_terminator(ec_level).unwrap();
        bits
    }

    fn render_barcode(&self, b_info: &BarcodeInfo, data: &[u8]) -> RgbImage {
        let bits = self.generate_barcode_filling_bits(b_info.version, b_info.ec_level, data);
        let code = QrCode::with_bits(bits, b_info.ec_level).unwrap();
        let code_image = code.render::<Rgb<u8>>().module_dimensions(1, 1).quiet_zone(false).build();
        code_image
    }

    pub fn encode(&self, out_image: &mut RgbImage, page_number: u16, is_parity_page: bool, parity_index: u16, file_checksum: u32, page_start_offset: u64, total_length: u64, data: &[u8]) {
        // Pulling this out into its own reference so we can pseudorandomize this later.
        let barcodes: &Vec<BarcodeInfo> = &self.cache_barcodes;

        // Fill the background with white so we don't have to do a quiet zone for each barcode individually.
        draw_filled_rect_mut(out_image, Rect::at(0, 0).of_size(out_image.width(), out_image.height()), Rgb([255, 255, 255]));

        let mut start_offset: usize = 0;
        for (b, b_info) in barcodes.iter().enumerate() {
            println!("Generating page {} barcode {}/{}", page_number, b, barcodes.len());
            // Pull in the amount of data we need to fill this barcode, padded out with zeroes.
            let mut barcode_data: Vec<u8> = vec!();

            // First byte - format version.
            barcode_data.push(self.format_version);

            // Next two bytes - page number, big endian.
            let page_number_bytes = page_number.to_be_bytes();
            barcode_data.extend_from_slice(&page_number_bytes);

            // Next two bytes - barcode number, big endian, with some metadata bits.
            let mut byte_1 = ((b >> 16) & 0x0f) as u8;
            let byte_2 = (b & 0xff) as u8;
            if is_parity_page {
                byte_1 |= 0b1000000;
            }
            barcode_data.push(byte_1);
            barcode_data.push(byte_2);

            // Next 6 bytes - offset from the start of the file, big endian.
            // TODO: This might need to be the parity page number we're encoding.
            let start_offset_bytes = page_start_offset.to_be_bytes();
            barcode_data.extend_from_slice(&start_offset_bytes[2..8]);

            // Next 6 bytes - total document length, big endian.
            let total_length_bytes = total_length.to_be_bytes();
            barcode_data.extend_from_slice(&total_length_bytes[2..8]);

            // Next 3 bytes - lower bytes document checksum, big endian.
            let checksum_bytes = file_checksum.to_be_bytes();
            barcode_data.extend_from_slice(&checksum_bytes[1..4]);

            let overhead = barcode_data.len();
            let data_capacity = (b_info.capacity as usize) - overhead;

            let mut v: Vec<u8>;
            let barcode_slice: &[u8];
            if (start_offset + data_capacity) < data.len() {
                // We can pull a full slice.
                barcode_slice = &data[start_offset..(start_offset + data_capacity)];
            }
            else if start_offset < data.len() {
                // We can pull a partial slice.
                v = data[(start_offset as usize)..data.len()].to_vec();
                v.resize(data_capacity, 0);
                barcode_slice = v.as_slice();
            }
            else {
                // We have to just use padding.
                v = vec![0 as u8; data_capacity];
                barcode_slice = v.as_slice();
            };
            barcode_data.extend_from_slice(barcode_slice);

            let code_image = self.render_barcode(&b_info, &barcode_data);
            imageops::overlay(out_image, &code_image, b_info.x, b_info.y);

            // Advance the offset for the next barcode.
            start_offset += b_info.capacity as usize;
        }
    }
}