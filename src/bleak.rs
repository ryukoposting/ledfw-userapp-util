use crate::elf2ble::{BleCommand, OutputFormat};

pub struct Bleak<W: std::io::Write> {
    out: W,
    prelude_written: bool,
}

impl<W: std::io::Write> From<W> for Bleak<W> {
    fn from(out: W) -> Self {
        Self {
            out,
            prelude_written: false,
        }
    }
}

const PRELUDE: &'static str = "
import asyncio
from bleak import BleakClient
import sys
import os

def gen_uuid(uuid):
    return f\"a277{uuid}-e035-13ae-4647-0e0437dd272a\"

ADDRESS = sys.argv[1]
# ADDRESS = \"dc:51:7a:fb:4a:3e\"   # for Evan's sanity

STATUS_UUID = gen_uuid(\"3040\")
PROG_UUID = gen_uuid(\"3101\")
APP_INFO_UUID = gen_uuid(\"3102\")
APP_NAME_UUID = gen_uuid(\"3103\")
APP_PROVIDER_UUID = gen_uuid(\"3104\")

PROGRAM = [
";

const CONCLUSION: &'static str = "
]

async def main():
    def notify_callback(sender: int, data: bytearray):
        print(f\"{sender}: {data}\")

    print(\"Scanning for\", ADDRESS)
    async with BleakClient(ADDRESS) as client:
        print(\"Connected. Enabling notifications...\")
        await client.start_notify(STATUS_UUID, notify_callback)
        await client.start_notify(APP_NAME_UUID, notify_callback)
        await client.start_notify(APP_PROVIDER_UUID, notify_callback)
        await client.start_notify(APP_INFO_UUID, notify_callback)
        await asyncio.sleep(0.5)
        print(\"Requesting app info...\")
        await client.write_gatt_char(PROG_UUID, b\"\\x00\") # update info characteristics
        await asyncio.sleep(1.0)
        print(\"Beginning installation...\")
        for i, packet in enumerate(PROGRAM):
            print(f'{i/len(PROGRAM)*100}%')
            await client.write_gatt_char(PROG_UUID, bytearray(packet))
        await asyncio.sleep(1.0)

asyncio.run(main())
";

impl<W: std::io::Write> OutputFormat for Bleak<W> {
    type Error = std::io::Error;
    type Finish = ();

    fn write_cmd<'s>(&mut self, cmd: BleCommand<'s>) -> Result<(), Self::Error> {
        if !self.prelude_written {
            self.prelude_written = true;
            self.out.write_all(PRELUDE.as_bytes())?;
        }

        match cmd {
            BleCommand::Start => writeln!(self.out, "  [2],"),
            BleCommand::Write { offset, data } => {
                let offset_bytes = offset.to_le_bytes();
                write!(self.out, "  [3, 0x{:02x}, 0x{:02x}, ", offset_bytes[0], offset_bytes[1])?;

                for byte in data.iter() {
                    write!(self.out, "0x{:02x}, ", byte)?;
                }

                writeln!(self.out, "],")
            },
            BleCommand::Commit { crc } => {
                let crc_bytes = crc.to_le_bytes();
                write!(self.out, "  [4, 0x{:02x}, 0x{:02x}],", crc_bytes[0], crc_bytes[1])
            },
            BleCommand::Run { crc } => {
                let crc_bytes = crc.to_le_bytes();
                write!(self.out, "  [5, 0x{:02x}, 0x{:02x}],", crc_bytes[0], crc_bytes[1])
            },
        }
    }

    fn finish(mut self) -> Result<(), Self::Error> {
        self.out.write_all(CONCLUSION.as_bytes())
    }

    fn mtu(&self) -> usize {
        124
    }
}
