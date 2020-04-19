extern crate clap;
use clap::{Arg, App};

mod stress_test_page;
mod archive_human_output_file;
use stress_test_page::StressTestPage;
use archive_human_output_file::{OutputFormat, ArchiveHumanOutputFile};

fn validate_integer(v: String) -> Result<(), String> {
    match v.parse::<u16>() {
        Ok(_n) => return Ok(()),
        Err(_) => return Err(String::from("Value given was not a positive integer."))
    }
}

fn validate_positive_float(v: String) -> Result<(), String> {
    match v.parse::<f32>() {
        Ok(n) => if n >= 0.0 { return Ok(()); } else { return Err(String::from("Value given was negative."))},
        Err(_) => return Err(String::from("Value given was not numeric."))
    }
}

fn main() {
    let matches = App::new("Real World Archive")
                    .version("0.0.1")
                    .author("Kyle Maas <kylemaasdev@gmail.com>")
                    .about("Archives data to a format suitable for printing or engraving.")
                    .arg(Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("File or directory to read input from.  Required unless running a stress test in encode mode.")
                        .takes_value(true)
                        .required_unless("stresstest")
                        .display_order(1))
                    .arg(Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("File or directory to place output in.  Required unless running a stress test in decode mode.")
                        .takes_value(true)
                        .required(true)
                        .display_order(2))
                    .arg(Arg::with_name("format")
                        .short("f")
                        .long("format")
                        .help("Output format to use.  Currently only \"png\" is supported, and is the default output format.")
                        .takes_value(true)
                        .possible_values(&["png"])
                        .default_value("png"))
                    .arg(Arg::with_name("units")
                        .short("u")
                        .long("units")
                        .help("Unit system to use for measurements.  Defaults to \"in\"")
                        .takes_value(true)
                        .possible_values(&["in", "mm", "px"])
                        .default_value("in"))
                    .arg(Arg::with_name("pagewidth")
                        .short("w")
                        .long("width")
                        .help("Page width, in real world units.  Defaults to \"8.5\"")
                        .takes_value(true)
                        .validator(validate_positive_float)
                        .default_value("8.5"))
                    .arg(Arg::with_name("pageheight")
                        .short("h")
                        .long("height")
                        .help("Page height, in real world units.  Defaults to \"11\"")
                        .takes_value(true)
                        .validator(validate_positive_float)
                        .default_value("11"))
                    .arg(Arg::with_name("margins")
                        .short("m")
                        .long("margins")
                        .help("Margins, specified as a space-separated list of top, right, bottom, left.  Defaults to \"0.25 0.25 0.5 0.25\"")
                        .takes_value(true)
                        .default_value("0.25 0.25 0.5 0.25"))
                    .arg(Arg::with_name("dpi")
                        .short("D")
                        .long("dpi")
                        .help("Target DPI.  Defaults to \"600\"")
                        .validator(validate_integer)
                        .default_value("600"))
                    .arg(Arg::with_name("decode")
                        .short("d")
                        .long("decode")
                        .help("Use this to decode the given filename.  Either encode or decode must be specified.")
                        .required_unless("encode")
                        .conflicts_with("encode")
                        .display_order(1))
                    .arg(Arg::with_name("encode")
                        .short("e")
                        .long("encode")
                        .help("Encode to the given filename as output.  Either encode or decode must be specified.")
                        .required_unless("decode")
                        .conflicts_with("decode")
                        .display_order(2))
                    .arg(Arg::with_name("parity")
                        .short("p")
                        .long("parity")
                        .help("Number of pages of parity to generate.  This equates to the number of full pages which can be lost from the rest of the document.  Defaults to \"0\"")
                        .default_value("0"))
                    .arg(Arg::with_name("stresstest")
                        .short("t")
                        .long("stresstest")
                        .help("Generate a stress test"))
                    .get_matches();
    let width = matches.value_of("pagewidth").unwrap();
    let height = matches.value_of("pageheight").unwrap();
    let dpi = matches.value_of("dpi").unwrap();
    let format = OutputFormat::PNG; //matches.value_of("format").unwrap().to_lowercase();
    if matches.is_present("encode") {
        // Encode.
        let out_file = matches.value_of("output").unwrap();
        if matches.is_present("stresstest") {
            // Generate a stress test page.
            let header = "Stress Test - {{dpi}} DPI, {{total_overlay_colors}}x Color Packing";
            let writer = ArchiveHumanOutputFile::new(out_file, format)
                .size(width.parse::<f32>().unwrap(), height.parse::<f32>().unwrap())
                .dpi(dpi.parse::<u16>().unwrap())
                .document_header(&header)
                .document_footer("Scan to test limits of your printing and scanning process")
                .total_pages(1)
                .finalize();
            let stresstest = StressTestPage::new(&writer)
                .finalize();
            stresstest.output();
        }
        else {
            // Encode normal data.
            let in_file = matches.value_of("input").unwrap();
        }
    }
    else {
        // Decode.
        let in_file = matches.value_of("input").unwrap();
        if matches.is_present("stresstest") {
            // Decode a stress test page.
        }
        else {
            // Decode normal data.
            let out_file = matches.value_of("output").unwrap();
        }
    }
}
