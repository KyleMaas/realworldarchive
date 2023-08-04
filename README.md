# RealWorldArchive

The goal of this project is to build a system where data files can be encoded in a machine-readable format which can be printed, laser etched, etc. for a long-lasting and durable data archival format.  Ideally, we want to keep this as simple and easily-reproducible as possible so that compatible systems can be written in several different languages and platforms for easy data interchange using a common format.

**This system is currently in very early development.  Expect breaking changes.**

## How it works

The system as currently described in the Design Outline generates pages of QR codes, optionally multiplexed into color for greater density, which can be printed out, archived, scanned, and decoded back to their original data.

## Usage

```
./realworldarchive --help
Real World Archive 0.0.1
Kyle Maas <kylemaasdev@gmail.com>
Archives data to a format suitable for printing or engraving.

USAGE:
    realworldarchive [FLAGS] [OPTIONS] --decode --encode --input <input> --output <output>

FLAGS:
    -d, --decode        Use this to decode the given filename.  Either encode or decode must be specified.
    -e, --encode        Encode to the given filename as output.  Either encode or decode must be specified.
        --help          Prints help information
    -t, --stresstest    Generate a stress test
    -V, --version       Prints version information

OPTIONS:
    -i, --input <input>              File or directory to read input from.  Required unless running a stress test in
                                     encode mode.
    -o, --output <output>            File or directory to place output in.  Required unless running a stress test in
                                     decode mode.
    -c, --colors <colors>            Maximum number of colors.  Defaults to "2" for monochrome [default: 2]
    -D, --dpi <dpi>                  Target DPI.  Defaults to "300" [default: 300]
        --ecfunction <ecfunction>    Error correction function for how much error correction to use for each barcode
                                     depending on its position on the page.  Defaults to "radial" to skew error
                                     correction so there is less in the center of the page and more toward the corners
                                     but can be set to "constant" for a constant level of error correction across the
                                     entire page [default: radial]  [possible values: constant, radial]
        --ecmax <ecmax>              Maximum percentage of error correction - just the number [0..100].  Please note
                                     this is not the amount of a barcode which can be lost and recovered but a
                                     percentage of the range we can run on.  For example, QR codes have a "0" level of
                                     7% error corraction and "100" level of 30% of data which can be recovered.  Only
                                     applicable in non-constant error correction functions.  Defaults to "100" [default:
                                     100]
        --ecmin <ecmin>              Minimum percentage of error correction - just the number [0..100].  Please note
                                     this is not the amount of a barcode which can be lost and recovered but a
                                     percentage of the range we can run on.  For example, QR codes have a "0" level of
                                     7% error corraction and "100" level of 30% of data which can be recovered.  If the
                                     constant error correction function is used, this is the amount used over the whole
                                     page.  Defaults to "25" [default: 25]
    -f, --format <format>            Output format to use.  Currently only "png" is supported, and is the default output
                                     format. [default: png]  [possible values: png]
    -m, --margins <margins>          Margins, specified as a space-separated list of top, right, bottom, left.  Defaults
                                     to "0.25 0.25 0.5 0.25" [default: 0.25 0.25 0.5 0.25]
    -h, --height <pageheight>        Page height, in real world units.  Defaults to "11" [default: 11]
    -w, --width <pagewidth>          Page width, in real world units.  Defaults to "8.5" [default: 8.5]
    -p, --parity <parity>            Number of pages of parity to generate.  This equates to the number of full pages
                                     which can be lost from the rest of the document.  Defaults to "0" [default: 0]
    -u, --units <units>              Unit system to use for measurements.  Defaults to "in" [default: in]  [possible
                                     values: in, mm, px]
```

So, for example, to encode a file:

```
./realworldarchive -e -D 72 -c 8 -i "Design outline.txt" -o "test_out/encodedfile.png"
```

And to decode:

```
./realworldarchive -d -c 8 -i "test_out/encodedfile.*.png" -o "test_out/reconstructed.txt"
```

## License

This project is triple-licensed under MIT, Apache-2.0 (or later), or zlib/libpng license.  You may use it under any of those licenses.

## Contributions

If you want to contribute to this project, you must agree that your contributions will be licensed as the rest of the project is and agree that your changes may be relicensed if the project goes in a different direction.  As an example, if in the future we decide to relicense this project or any specification described herein as Creative Commons CC-0, you agree that we may relicense your code under a different license.

## History

After realizing that nearly every data format I have available to me is susceptible in some form to EMP or other strong electromagnetic interference (hard disks and flash memory, for example), I figured there had to be some way to make data archival more disaster-resistant.  Sure, you could Base64-encode your data, print it out, and then try to OCR it later.  But the data density of that is quite a bit lower than what should be possible, and without some kind of correction system, it relies on recognition being perfect for the output to be exactly what the input was.  Plus, if there was any damage, it could not be automatically repaired.  Thus, this system came about, to try to provide additional data durability and automatic recovery in the event of misrecognition or physical damage.

Initial work on this started in 2020, which is when I had most of the format spec pretty well figured out.  At the time, I had not heard of HCC2D for color multiplexing or the system from mit41301 - [HackADay](https://hackaday.com/2023/07/28/color-can-triple-qr-code-capacity/) or [HackADay.io](https://hackaday.io/project/192082-rectangular-micro-qr-code-rmqr) - so the color multiplexing system this uses is a bit different.  If HCC2D ends up advancing past a prototype stage, we may switch to that in the future purely for standards-compliance and easier reimplementation of this system in other languages and on other platforms.  And mit41301's system is probably very durable but lower-density than is useful for this project.  But I would like to acknowledge that others are working in this space and may come up with something better than what we have here.

## Roadmap

This system is currently in the "working prototype" phase.  It works as-is, but it does not fully implement all of the data correction technologies in the Design Outline.  Moving forward, the goal of this project is to fully implement that so that data can be stored in an even more durable manner.