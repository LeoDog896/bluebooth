extern crate itertools;

use bluer::{Adapter, AdapterEvent, Address, DeviceEvent, AddressType};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::HashSet, env};
use tokio::sync::Mutex;
use cli_table::{Table, print_stdout, WithTitle};
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Table)]
struct BlueboothDevice {
    #[table(title = "Address")]
    address: AddressType,
    #[table(title = "Name")]
    name: String,
    #[table(title = "Icon")]
    icon: String,
    #[table(title = "Class")]
    class: String,
    #[table(title = "UUID")]
    uuid: String,
    #[table(title = "Paired")]
    paired: bool,
    #[table(title = "Connected")]
    connected: bool,
    #[table(title = "Trusted")]
    trusted: bool,
    #[table(title = "Modalias")]
    modalias: String,
    #[table(title = "RSSI")]
    rssi: String,
    #[table(title = "TX Power")]
    tx_power: String,
    #[table(title = "Manufacturer Data")]
    manufacturer_data: String,
    #[table(title = "Service Data")]
    service_data: String
}

async fn to_blueboot_device(adapter: &Adapter, addr: Address) -> bluer::Result<BlueboothDevice> {
    let device = adapter.device(addr)?;
    Ok(BlueboothDevice {
        address: device.address_type().await?,
        name: device.name().await?.unwrap_or("None".to_string()),
        icon: device.icon().await?.unwrap_or("None".to_string()),
        class: device.class().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        uuid: device.uuids().await?.map(|it| itertools::join(&it, ", ")).unwrap_or("None".to_string()),
        paired: device.is_paired().await?,
        connected: device.is_connected().await?,
        trusted: device.is_trusted().await?,
        modalias: device.modalias().await?.map(|it| format!("{:?}", it)).unwrap_or("None".to_string()),
        rssi: device.rssi().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        tx_power: device.tx_power().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        manufacturer_data: device.tx_power().await?.map(|it| it.to_string()).unwrap_or("None".to_string()),
        service_data: device.service_data().await?
            .map(|it| it.iter().map(|(k, v)| format!("{}: {}", k, itertools::join(v, ", "))).collect())
            .map(|it: HashSet<String>| itertools::join(&it, ", ")).unwrap_or("None".to_string()),
    })
}

type SafeBlueboothDeviceMap = Arc<Mutex<HashMap<Address, BlueboothDevice>>>;

fn print_devices<T: Table>(devices: T) -> std::io::Resuit<()> {
    print_stdout(devices.with_title())?;
    println!();
}

async fn add_device(address: Address, device: BlueboothDevice, devices: SafeBlueboothDeviceMap) -> std::io::Result<()> {
    let mut editable_devices = devices.lock().await;
    editable_devices.insert(address, device);


    print_stdout(editable_devices.values().with_title())?;
    println!();

    Ok(())
}

async fn remove_device(address: Address, devices: SafeBlueboothDeviceMap) -> std::io::Result<()> {
    let mut editable_devices = devices.lock().await;
    editable_devices.remove(&address);

    print_stdout(editable_devices.values().with_title())?;
    println!();

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let devices: SafeBlueboothDeviceMap = Arc::new(Mutex::new(HashMap::new()));

    let with_changes = env::args().any(|arg| arg == "--changes");
    let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    env_logger::init();
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

                        let device = to_blueboot_device(&adapter, addr).await?;

                        add_device(addr, device, devices.clone()).await?;
                        

                        if with_changes {
                            let device = adapter.device(addr)?;
                            let change_events = device.events().await?.map(move |evt| (addr, evt));
                            all_change_events.push(change_events);
                        }
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        remove_device(addr, devices.clone()).await?;
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