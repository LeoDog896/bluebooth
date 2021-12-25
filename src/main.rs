extern crate itertools;
#[macro_use] extern crate prettytable;

use bluer::{Adapter, AdapterEvent, Address, DeviceEvent};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::HashSet, env};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use prettytable::{Row, format};
use terminal_emoji::Emoji;

fn bool_to_emoji(flag: bool) -> Emoji<'static> {
    if flag {
        Emoji::new("✅", "true")
    } else {
        Emoji::new("❌", "false")
    }
}

async fn to_bluetooth_info(adapter: &Adapter, addr: Address) -> bluer::Result<Row> {

    let device = adapter.device(addr)?;
    Ok(row![
        Fb->device.address_type().await?.to_string(),
        Fy->device.name().await?.unwrap_or("".to_string()),
        Fb->device.icon().await?.unwrap_or("".to_string()),
        Fy->device.class().await?.map(|it| it.to_string()).unwrap_or("".to_string()),
        Fb->device.uuids().await?.map(|it| itertools::join(&it, ", ")).unwrap_or("".to_string()),
        Fyc->bool_to_emoji(device.is_paired().await?),
        Fbc->bool_to_emoji(device.is_connected().await?),
        Fyc->bool_to_emoji(device.is_trusted().await?),
        Fb->device.modalias().await?.map(|it| format!("{:?}", it)).unwrap_or("".to_string()),
        Fy->device.rssi().await?.map(|it| it.to_string()).unwrap_or("".to_string()),
        Fb->device.tx_power().await?.map(|it| it.to_string()).unwrap_or("".to_string()),
        Fy->device.service_data().await?
            .map(|it| it.iter().map(|(_, v)| format!("{}", itertools::join(v, ", "))).collect())
            .map(|it: HashSet<String>| itertools::join(&it, ", ")).unwrap_or("".to_string()),
        Fb->device.manufacturer_data().await?
            .map(|it| it.iter().map(|(k, v)| format!("{}: {}", k, itertools::join(v, ", "))).collect())
            .map(|it: HashSet<String>| itertools::join(&it, ", ")).unwrap_or("".to_string()),
    ])
}

type ThreadSafeBlueboothDeviceMap = Arc<RwLock<HashMap<Address, Row>>>;

async fn print_table(devices: ThreadSafeBlueboothDeviceMap) {
    let editable_devices = devices.read().await;

    let mut table = table!([
        FBc->"Address",
        FYc->"Name",
        FBc->"Icon",
        FYc->"Class",
        FBc->"UUIDs",
        FYc->"Paired",
        FBc->"Connected",
        FYc->"Trusted",
        FBc->"Modalias",
        FYc->"RSSI",
        FBc->"Tx Power",
        FYc->"Service Data",
        FBc->"Manufacturer Data"
    ]);

    let format = *format::consts::FORMAT_BOX_CHARS;
    table.set_format(format);

    for device in editable_devices.values() {
        table.add_row(device.to_owned());
    };

    table.printstd();
}

async fn set_info(address: Address, device_info: Row, devices: ThreadSafeBlueboothDeviceMap) -> std::io::Result<()> {
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

                        let device = adapter.device(addr)?;
                        let change_events = device.events().await?.map(move |evt| (addr, evt));
                        all_change_events.push(change_events);
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