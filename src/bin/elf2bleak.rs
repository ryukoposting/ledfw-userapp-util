use std::{path::Path, process::exit, env};

use elf::File;

use userapputil::{elf2ble::ElfToBle, bleak::Bleak};

fn main() {
    if env::args().len() != 4 {
        println!("Usage: elf2bleak <run | commit> input.elf output.py");
        exit(0);
    }

    let mode = env::args().nth(1)
        .expect("Missing mode");

    let do_commit = if mode.as_str() == "commit" {
        true
    } else if mode.as_str() == "run" {
        false
    } else {
        println!("Invalid argument {}, expected \"run\" or \"commit\"", mode);
        exit(-1);
    };

    let inpath = env::args().nth(2)
        .expect("Missing input ELF file");
    let outpath = env::args().nth(3)
        .expect("Missing output PY file");

    let inpath = Path::new(inpath.as_str());
    let outpath = Path::new(outpath.as_str());

    let file = File::open_path(inpath).expect("open_path");

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

    to_ble.generate(&file, bleak, do_commit).expect("Generate");
}
