use std::fmt::Display;
use elf::{File, Section};
use crc16::{CrcType, CCITT_FALSE};

pub mod bleak;
pub mod direct;

pub struct ElfToBle {
    pub text_addr: u32,
    pub text_len: u32,
    pub data_addr: u32,
    pub data_len: u32,
}

#[derive(Debug)]
pub enum Command<'s> {
    Start,
    Write { offset: u16, data: &'s [u8] },
    Commit { crc: u16 },
}

#[derive(Debug)]
pub enum Error<'s> {
    MissingSection(&'static str),
    IncorrectSectionLocation { section: &'s Section, expected: u32 },
    SectionTooLarge { section: &'s Section, max: u32 },
    MissingMagicNumber,
}

pub trait OutputFormat {
    type Error: std::error::Error;

    fn mtu(&self) -> usize;

    fn write_cmd<'s>(&mut self, cmd: Command<'s>) -> Result<(), Self::Error>;

    fn write_all_cmds<'s>(&mut self, cmds: Vec<Command<'s>>) -> Result<(), Self::Error> {
        for cmd in cmds {
            self.write_cmd(cmd)?;
        }
        Ok(())
    }

    fn finish(self) -> Result<(), Self::Error>;
}

impl ElfToBle {
    pub fn validate<'s>(&self, file: &'s File) -> Result<(), Error<'s>> {
        let section = file
            .get_section(".text")
            .ok_or(Error::MissingSection(".text"))?;

        if section.shdr.addr != self.text_addr as u64 {
            return Err(Error::IncorrectSectionLocation {
                section,
                expected: self.text_addr,
            });
        }

        if section.shdr.size > self.text_len as u64 {
            return Err(Error::SectionTooLarge {
                section,
                max: self.text_addr + self.text_len,
            });
        }

        if let Some(section) = file.get_section(".bss") {
            if section.shdr.addr < self.data_addr as u64 {
                return Err(Error::IncorrectSectionLocation {
                    section,
                    expected: self.data_addr,
                });
            }

            if section.shdr.addr > (self.data_addr + self.data_len) as u64 {
                return Err(Error::IncorrectSectionLocation {
                    section,
                    expected: self.data_addr,
                });
            }

            if section.shdr.addr + section.shdr.size > (self.data_addr + self.data_len) as u64 {
                return Err(Error::SectionTooLarge {
                    section,
                    max: self.data_addr + self.data_len,
                });
            }
        }

        let section = file
            .get_section(".data")
            .ok_or(Error::MissingSection(".data"))?;

        if section.shdr.addr < self.data_addr as u64 {
            return Err(Error::IncorrectSectionLocation {
                section,
                expected: self.data_addr,
            });
        }

        if section.shdr.addr > (self.data_addr + self.data_len) as u64 {
            return Err(Error::IncorrectSectionLocation {
                section,
                expected: self.data_addr,
            });
        }

        if section.shdr.addr + section.shdr.size > (self.data_addr + self.data_len) as u64 {
            return Err(Error::SectionTooLarge {
                section,
                max: self.data_addr + self.data_len,
            });
        }

        self.validate_data_section(section)?;

        Ok(())
    }

    fn validate_data_section<'s>(&self, data: &'s Section) -> Result<(), Error<'s>> {
        if data.data.len() < 4 {
            return Err(Error::MissingMagicNumber);
        }

        let magic = u32::from_le_bytes(data.data[..4].try_into().unwrap());

        if magic != 0x00041198 {
            return Err(Error::MissingMagicNumber);
        }

        Ok(())
    }

    pub fn generate<'s, O: OutputFormat>(&self, file: &'s File, mut out: O) -> Result<(), O::Error> {
        let mut result = vec![Command::Start];
        let mut crc = CCITT_FALSE::init();
        let data = file.get_section(".data").expect("data");
        let code = file.get_section(".text").expect("text");
        // let bss = file.get_section(".bss");

        // data must come first, followed by bss, then code
        let mut addr = 0u32;

        for chunk in data.data.chunks(out.mtu() - 3) {
            // println!("0x{:04x} {:02x?}", addr, chunk);
            result.push(Command::Write {
                offset: addr as u16,
                data: chunk,
            });
            addr += chunk.len() as u32;
            crc = CCITT_FALSE::update(crc, chunk);
        }

        // entire write buffer is already cleared, so we don't need to generate
        // commands for .bss
        // if let Some(bss) = bss {
        //     let next_addr = bss.shdr.addr as u32 - self.data_addr;
        //     while addr < next_addr {
        //         crc = CCITT_FALSE::update(crc, &[0]);
        //         addr += 1;
        //     }

        //     for chunk in bss.data.chunks(MAX_WRITE_BUF) {
        //         // println!("0x{:04x} {:02x?}", addr, chunk);
        //         result.push(Command::Write { offset: addr as u16, data: chunk });
        //         addr += chunk.len() as u32;
        //         crc = CCITT_FALSE::update(crc, chunk);
        //     }
        // }

        let next_addr = code.shdr.addr as u32 - self.data_addr;
        while addr < next_addr {
            crc = CCITT_FALSE::update(crc, &[0]);
            addr += 1;
        }

        for chunk in code.data.chunks(out.mtu() - 3) {
            // println!("0x{:04x} {:02x?}", addr, chunk);
            result.push(Command::Write {
                offset: addr as u16,
                data: chunk,
            });
            addr += chunk.len() as u32;
            crc = CCITT_FALSE::update(crc, chunk);
        }

        let last_addr = self.data_len + self.text_len;
        while addr < last_addr {
            crc = CCITT_FALSE::update(crc, &[0]);
            addr += 1;
        }

        crc = CCITT_FALSE::get(crc);

        println!("CRC=0x{:04x}", crc);

        result.push(Command::Commit { crc });

        out.write_all_cmds(result)?;
        out.finish()
    }
}

impl<'s> Display for Error<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingSection(name) => write!(f, "Missing section: {}", name),
            Error::IncorrectSectionLocation { section, expected } => write!(
                f,
                "Section '{}' is located at 0x{:08x}, but should be at 0x{:08x}",
                section.shdr.name, section.shdr.addr, expected
            ),
            Error::SectionTooLarge { section, max } => write!(
                f,
                "Section '{}' overflowed by {} bytes",
                section.shdr.name,
                section.shdr.addr + section.shdr.size - *max as u64
            ),
            Error::MissingMagicNumber => write!(
                f,
                "Missing magic number - did you forget DEFINE_LED_VECTBL?"
            ),
        }
    }
}

impl<'s> std::error::Error for Error<'s> {}
