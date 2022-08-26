pub mod bleak;
pub mod elf2ble;

use std::path::PathBuf;

use btleplug::api::BDAddr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(
        subcommand,
    )]
    pub command: Command,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Utilities for Ledx user-layer apps")]
pub enum Command {
    #[structopt(about = "Generate LXML file")]
    Lxml {
        #[structopt(short, long, parse(from_os_str))]
        input: PathBuf,
        #[structopt(short, long, parse(from_os_str))]
        output: PathBuf,
    },
    #[structopt(about = "Run a program on a device")]
    Run {
        #[structopt(short, long, parse(from_os_str))]
        input: PathBuf,
        #[structopt(short, long, required_unless = "mac")]
        name: Option<String>,
        #[structopt(short, long, required_unless = "name", parse(try_from_str = BDAddr::from_str_delim))]
        mac: Option<BDAddr>,
    },
    #[structopt(about = "Install a program onto a device")]
    Install {
        #[structopt(short, long, parse(from_os_str))]
        input: PathBuf,
        #[structopt(short, long, required_unless = "mac")]
        name: Option<String>,
        #[structopt(short, long, required_unless = "name", parse(try_from_str = BDAddr::from_str_delim))]
        mac: Option<BDAddr>,
    },
}
