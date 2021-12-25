extern crate itertools;
#[macro_use] extern crate prettytable;

use bluer::{Adapter, AdapterEvent, Address, DeviceEvent};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::HashSet, env};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use prettytable::{Row, Cell};

async fn to_bluetooth_info(adapter: &Adapter, addr: Address) -> bluer::Result<Vec<String>> {
    let device = adapter.device(addr)?;
    Ok(vec![
        device.address_type().await?.to_string(),
        device.name().await?.unwrap_or("None".to_string()),
        device.icon().await?.unwrap_or("None".to_string()),
        device.class().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        device.uuids().await?.map(|it| itertools::join(&it, ", ")).unwrap_or("None".to_string()),
        device.is_paired().await?.to_string(),
        device.is_connected().await?.to_string(),
        device.is_trusted().await?.to_string(),
        device.modalias().await?.map(|it| format!("{:?}", it)).unwrap_or("None".to_string()),
        device.rssi().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        device.tx_power().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        device.tx_power().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        device.service_data().await?
            .map(|it| it.iter().map(|(k, v)| format!("{}: {}", k, itertools::join(v, ", "))).collect())
            .map(|it: HashSet<String>| itertools::join(&it, ", ")).unwrap_or("None".to_string()),
    ])
}

type ThreadSafeBlueboothDeviceMap = Arc<RwLock<HashMap<Address, Vec<String>>>>;

async fn print_table(devices: ThreadSafeBlueboothDeviceMap) {
    let editable_devices = devices.read().await;

    let mut table = table!(
        ["Address", "Name", "Icon", "Class", "UUIDs", "Paired", "Connected", "Trusted", "Modalias", "RSSI", "Tx Power", "Service Data"]
    );

    editable_devices.values().for_each(|device| {
        table.add_row(Row::new(device.iter().map(|it| Cell::new(&it)).collect()));
    });

    table.printstd();
}

async fn set_info(address: Address, device_info: Vec<String>, devices: ThreadSafeBlueboothDeviceMap) -> std::io::Result<()> {
    let mut editable_devices = devices.write().await;
    editable_devices.insert(address, device_info);

    Ok(())
}

async fn remove_info(address: Address, devices: ThreadSafeBlueboothDeviceMap) -> std::io::Result<()> {
    let mut editable_devices = devices.write().await;
    editable_devices.remove(&address);

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let devices: ThreadSafeBlueboothDeviceMap = Arc::new(RwLock::new(HashMap::new()));

    let with_changes = env::args().any(|arg| arg == "--changes");
    let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    let session = bluer::Session::new().await?;
    let adapter_names = session.adapter_names().await?;
    let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    println!("Discovering devices using Bluetooth adapater {}\n", &adapter_name);
    let adapter = session.adapter(adapter_name)?;
    adapter.set_powered(true).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        if !filter_addr.is_empty() && !filter_addr.contains(&addr) {
                            continue;
                        }

                        let device = to_bluetooth_info(&adapter, addr).await?;

                        set_info(addr, device, devices.clone()).await?;
                        print_table(devices.clone()).await;

                        if with_changes {
                            let device = adapter.device(addr)?;
                            let change_events = device.events().await?.map(move |evt| (addr, evt));
                            all_change_events.push(change_events);
                        }
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        remove_info(addr, devices.clone()).await?;
                        print_table(devices.clone()).await;
                    }
                    _ => (),
                }
                println!();
            }
            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                println!("Device changed: {}", addr);
                println!("    {:?}", property);
            }
            else => break
        }
    }

    Ok(())
}