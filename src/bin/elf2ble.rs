use std::{path::Path, process::exit};

use elf::File;

use elf2ble::{ElfToBle, bleak::Bleak};

fn main() {
    let path = Path::new("leds.elf");
    let outpath = Path::new("upload.py");

    let file = File::open_path(path).expect("open_path");

    let to_ble = ElfToBle {
        text_addr: 0x2003F800,
        text_len: 0x800,
        data_addr: 0x2003F000,
        data_len: 0x800
    };

    if let Err(err) = to_ble.validate(&file) {
        println!("Error: {}", err);
        exit(1);
    }

    let outfile = std::fs::File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open(outpath)
        .expect("open_output");

    let bleak = Bleak::from(outfile);

    to_ble.generate(&file, bleak).expect("Generate");
}
