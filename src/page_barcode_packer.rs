use image::Rgb;
use qrcode::bits::Bits;
use qrcode::types::{Version, EcLevel, Mode};

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
        if !self.packing_cached {
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
            while next_y < self.width {
                let dl = (self.damage_likelihood_map)((next_x + barcode_size / 2) as f32 / self.width as f32, (next_y + barcode_size / 2) as f32 / self.height as f32);
                let ec = {
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
                    }
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
    }

    pub fn data_bytes_per_page(self) -> u32 {
        self.cache_bytes_per_page
    }
}