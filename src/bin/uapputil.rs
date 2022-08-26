use std::{convert::Infallible, error::Error, process::exit, io::Write};

use btleplug::{platform::{Manager, Adapter, PeripheralId}, api::{Manager as _, Peripheral as _, ScanFilter, Central, Peripheral, BDAddr}};
use structopt::StructOpt;
use tokio::time;
use userapputil::{
    elf2ble::{BleCommand, ElfToBle, OutputFormat},
    Args,
    Command::{Install, Run},
};
use uuid::Uuid;

struct BleOutput {
    writes: Vec<Vec<u8>>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = Args::from_args().command;

    let do_commit = matches!(command, Install { .. });

    match &command {
        Run { input, name, mac } |
        Install { input, name, mac } => {
            let manager = Manager::new().await?;
            let adapter_list = manager.adapters().await?;
            if adapter_list.is_empty() {
                eprintln!("No Bluetooth adapters found");
            }

            if input.extension().map(|ex| ex == "elf").unwrap_or(false) {
                let infile = elf::File::open_path(input).expect("open");

                let to_ble = ElfToBle {
                    text_addr: 0x2003F800,
                    text_len: 0x800,
                    data_addr: 0x2003F000,
                    data_len: 0x800,
                };

                if let Err(err) = to_ble.validate(&infile) {
                    eprintln!("Error: {}", err);
                    exit(1);
                }

                let ble = BleOutput::new();
                let writes = to_ble.generate(&infile, ble, do_commit).expect("generate");
                println!("Scanning for target device.");

                for adapter in adapter_list.iter() {
                    if run_writes(adapter, &writes, name, mac).await? {
                        break;
                    }
                }
            } else {
                eprintln!(
                    "Invalid file extension (must be .elf or .lxml): {:?}",
                    input
                );
                exit(-1);
            }
        }
        _ => todo!(),
    };

    Ok(())
}

async fn run_writes(adapter: &Adapter, writes: &Vec<Vec<u8>>, name: &Option<String>, mac: &Option<BDAddr>) -> Result<bool,Box<dyn Error>> {
    println!("Starting scan on {}...", adapter.adapter_info().await?);
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");
    time::sleep(time::Duration::from_secs(10)).await;

    let peripherals = adapter.peripherals().await?;

    for peripheral in peripherals.iter() {
        let properties = peripheral.properties().await?.unwrap();
        let periph_mac = peripheral.address();

        let periph_name = if let Some(name) = properties.local_name {
            name
        } else {
            continue;
        };

        let name_matches = name.as_ref().map(|n| n.eq(&periph_name)).unwrap_or(true);
        let mac_matches = mac.as_ref().map(|m| m.eq(&periph_mac)).unwrap_or(true);

        if name_matches && mac_matches {
            println!("Connecting to {} ({})", periph_name, periph_mac);
            let did_upload = upload(peripheral, writes).await;
            if peripheral.is_connected().await? {
                peripheral.disconnect().await?;
            }
            if did_upload? {
                return Ok(true)
            }
        }
    }

    Ok(false)
}

async fn upload<P: Peripheral>(peripheral: &P, writes: &Vec<Vec<u8>>) -> Result<bool,Box<dyn Error>> {
    let app_svc_uuid = Uuid::parse_str("a2773100-e035-13ae-4647-0e0437dd272a").unwrap();
    let prog_char_uuid = Uuid::parse_str("a2773101-e035-13ae-4647-0e0437dd272a").unwrap();

    peripheral.connect().await?;

    time::sleep(time::Duration::from_secs(2)).await;

    peripheral.discover_services().await?;

    let services = peripheral.services();

    let service = services.iter()
        .filter(|svc| svc.uuid == app_svc_uuid)
        .nth(0);

    let service = if let Some(service) = service {
        service
    } else {
        eprintln!("Warning: Device was found, but the userapp service was not found.");
        return Ok(false)
    };

    let characteristic = service.characteristics.iter()
        .filter(|chr| chr.uuid == prog_char_uuid)
        .nth(0);

    let characteristic = if let Some(characteristic) = characteristic {
        characteristic
    } else {
        eprintln!("Warning: Device was found, but the userapp programming characteristic was not found.");
        return Ok(false)
    };

    print!("Uploading:");
    std::io::stdout().flush().unwrap();
    for (i, data) in writes.iter().enumerate() {
        let completion = (i + 1) as f64 * 100.0 / (writes.len() as f64);
        print!("\rUploading: {completion:.01}%");
        std::io::stdout().flush().unwrap();
        peripheral.write(characteristic, data.as_slice(), btleplug::api::WriteType::WithResponse).await?;
    }

    Ok(true)
}

impl BleOutput {
    fn new() -> Self {
        Self { writes: vec![] }
    }
}

impl OutputFormat for BleOutput {
    type Error = Infallible;
    type Finish = Vec<Vec<u8>>;

    fn mtu(&self) -> usize {
        120
    }

    fn write_cmd<'s>(&mut self, cmd: BleCommand<'s>) -> Result<(), Self::Error> {
        let w = match cmd {
            BleCommand::Start => vec![2u8],
            BleCommand::Write { offset, data } => {
                let offset = offset.to_le_bytes();
                let mut w = vec![3u8, offset[0], offset[1]];
                w.extend(data);
                w
            }
            BleCommand::Commit { crc } => {
                let crc = crc.to_le_bytes();
                vec![4, crc[0], crc[1]]
            }
            BleCommand::Run { crc } => {
                let crc = crc.to_le_bytes();
                vec![5, crc[0], crc[1]]
            }
        };
        self.writes.push(w);
        Ok(())
    }

    fn finish(self) -> Result<Self::Finish, Self::Error> {
        Ok(self.writes)
    }
}
