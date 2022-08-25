use std::error::Error;
use std::time::Duration;
use tokio::time;

use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        println!("Starting scan on {}...", adapter.adapter_info().await?);
        adapter
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(10)).await;
        let peripherals = adapter.peripherals().await?;
        if peripherals.is_empty() {
            eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
        } else {
            // All peripheral devices in range
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?.unwrap();
                let is_connected = peripheral.is_connected().await?;
                let local_name = properties
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                println!("Peripheral {:?}", local_name);

                for (k, v) in properties.manufacturer_data.iter() {
                    println!("{:?} = {:?}", k, v);
                }

                // if !is_connected {
                //     println!("Connecting to peripheral {:?}...", &local_name);
                //     if let Err(err) = peripheral.connect().await {
                //         eprintln!("Error connecting to peripheral, skipping: {}", err);
                //         continue;
                //     }
                // }
                // let is_connected = peripheral.is_connected().await?;
                // println!(
                //     "Now connected ({:?}) to peripheral {:?}...",
                //     is_connected, &local_name
                // );
                // peripheral.discover_services().await?;
                // println!("Discover peripheral {:?} services...", &local_name);
                // for service in peripheral.services() {
                //     println!(
                //         "Service UUID {}, primary: {}",
                //         service.uuid, service.primary
                //     );
                //     for characteristic in service.characteristics {
                //         println!("  {:?}", characteristic);
                //     }
                // }
                // if is_connected {
                //     println!("Disconnecting from peripheral {:?}...", &local_name);
                //     peripheral
                //         .disconnect()
                //         .await
                //         .expect("Error disconnecting from BLE peripheral");
                // }
            }
        }
    }
    Ok(())
}
