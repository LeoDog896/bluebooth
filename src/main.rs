extern crate itertools;
#[macro_use]
extern crate prettytable;

mod device_processor;

use anyhow::{Context, Error, Result};
use bluer::{Adapter, AdapterEvent, Address, Device, DeviceEvent, DeviceProperty, AddressType, Modalias};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use prettytable::{format, Row};
use single::Single;
use std::collections::HashMap;
use std::sync::Arc;
use std::{collections::HashSet, env};
use terminal_emoji::Emoji;
use tokio::sync::RwLock;

fn bool_to_emoji<'a>(flag: bool) -> Emoji<'a> {
    if flag {
        // Checkmark Emoji
        Emoji::new("\u{2705}", "yes")
    } else {
        // X emoji
        Emoji::new("\u{274c}", "no")
    }
}

fn get_device(adapter: &Adapter, addr: Address) -> bluer::Result<Device> {
    adapter.device(addr)
}

async fn to_bluetooth_info(device: &Device) -> Result<Row> {
    let service_data = device.service_data().await?;

    let service_data_string = service_data
        .map(|data| {
            if data.len() == 1 {
                data.values()
                    .single()
                    .map_err(Error::msg)
                    .context("Could not get first element even though array length is one")
                    .map(|v| itertools::join(v, ", "))
            } else {
                Ok(itertools::join(
                    data.iter()
                        .map(|(k, v)| format!("{:?}: {}", k, itertools::join(v, ", "))),
                    ", ",
                ))
            }
        })
        .unwrap_or_else(|| Ok("".to_string()))?;

    Ok(row![
        Fb->device.address().to_string(),
        Fy->device.name().await?.unwrap_or_else(|| "".to_string()),
        Fb->device.icon().await?.unwrap_or_else(|| "".to_string()),
        Fy->device.class().await?.map(|it| it.to_string()).unwrap_or_else(|| "".to_string()),
        Fb->device.uuids().await?.map(|it| itertools::join(&it, ", ")).unwrap_or_else(|| "".to_string()),
        Fyc->bool_to_emoji(device.is_paired().await?),
        Fbc->bool_to_emoji(device.is_connected().await?),
        Fyc->bool_to_emoji(device.is_trusted().await?),
        Fb->device.modalias().await?.map(|it| format!("{:?}", it)).unwrap_or_else(|| "".to_string()),
        Fy->device.rssi().await?.map(|it| it.to_string()).unwrap_or_else(|| "".to_string()),
        Fb->device.tx_power().await?.map(|it| it.to_string()).unwrap_or_else(|| "".to_string()),
        Fy->service_data_string,
        Fb->device.manufacturer_data().await?
            .map(|it| it.iter().map(|(k, v)| format!("{}: {}", k, itertools::join(v, ", "))).collect())
            .map_or_else(|| "".to_string(), |it: HashSet<String>| itertools::join(&it, ", ")),
    ])
}

type ThreadSafeBlueboothDeviceMap = Arc<
    RwLock<
        // The devices address (for easy lookup) to the Row containing its data,
        HashMap<Address, Device>,
    >,
>;

async fn print_table(devices: ThreadSafeBlueboothDeviceMap) -> Result<()> {
    let editable_devices = devices.read().await;

    let mut table = table!([
        BBc->"Address",
        BYc->"Name",
        BBc->"Icon",
        BYc->"Class",
        BBc->"UUIDs",
        BYc->"Paired",
        BBc->"Connected",
        BYc->"Trusted",
        BBc->"Modalias",
        BYc->"RSSI",
        BBc->"Tx Power",
        BYc->"Service Data",
        Bc->"Manufacturer Data"
    ]);

    let format = *format::consts::FORMAT_BOX_CHARS;
    table.set_format(format);

    for device in editable_devices.values() {
        table.add_row(to_bluetooth_info(device).await?.clone());
    }

    table.printstd();

    Ok(())
}

async fn set_info(
    address: Address,
    device: Device,
    devices: ThreadSafeBlueboothDeviceMap,
) -> std::io::Result<()> {
    let mut writable_devices = devices.write().await;
    writable_devices.insert(address, device);

    Ok(())
}

async fn remove_info(
    address: Address,
    devices: ThreadSafeBlueboothDeviceMap,
) -> std::io::Result<()> {
    let mut writable_devices = devices.write().await;
    writable_devices.remove(&address);

    Ok(())
}

async fn change_info(
    address: Address,
    devices: ThreadSafeBlueboothDeviceMap,
    property: DeviceProperty,
) -> Result<()> {
    let readable_devices = devices.read().await;

    let device = match readable_devices.get(&address) {
        None => return Ok(()),
        Some(x) => x,
    };

    let devices = devices.clone();
    let mut writable_devices = devices.write().await;

    match property {
        DeviceProperty::Name(name) => device.set_alias(name).await?,
        DeviceProperty::AddressType(address_type) => todo!(),
        DeviceProperty::Icon(icon) => todo!(),
        DeviceProperty::Class(class) => todo!(),
        DeviceProperty::Appearance(appearance) => todo!(),
        DeviceProperty::Uuids(uuids) => todo!(),
        DeviceProperty::Paired(paired) => todo!(),
        DeviceProperty::Connected(connected) => todo!(),
        DeviceProperty::Trusted(trusted) => todo!(),
        DeviceProperty::Blocked(blocked) => todo!(),
        DeviceProperty::WakeAllowed(wake_allowed) => todo!(),
        DeviceProperty::Alias(alias) => device.set_alias(alias).await?,
        DeviceProperty::LegacyPairing(legacy_pairing) => todo!(),
        DeviceProperty::Modalias(modalias) => todo!(),
        DeviceProperty::Rssi(rssi) => todo!(),
        DeviceProperty::TxPower(tx_power) => todo!(),
        DeviceProperty::ManufacturerData(manufacturer_data) => todo!(),
        DeviceProperty::ServiceData(service_data) => todo!(),
        DeviceProperty::ServicesResolved(services_resolved) => todo!(),
        DeviceProperty::AdvertisingFlags(advertising_flags) => todo!(),
        DeviceProperty::AdvertisingData(advertising_data) => todo!(),
    };

    writable_devices.insert(address, device.clone());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let devices: ThreadSafeBlueboothDeviceMap = Arc::new(RwLock::new(HashMap::new()));

    let filter_addr: HashSet<_> = env::args()
        .filter_map(|arg| arg.parse::<Address>().ok())
        .collect();

    let session = bluer::Session::new().await?;
    let adapter_names = session.adapter_names().await?;
    let adapter_name = adapter_names
        .first()
        .context("No Bluetooth adapter present")?;
    println!(
        "Discovering devices using Bluetooth adapater {}\n",
        &adapter_name
    );
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

                        let device = get_device(&adapter, addr)?;

                        set_info(addr, device, devices.clone()).await?;
                        print_table(devices.clone()).await?;

                        let device = adapter.device(addr)?;
                        let change_events = device.events().await?.map(move |evt| (addr, evt));
                        all_change_events.push(change_events);
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        remove_info(addr, devices.clone()).await?;
                        print_table(devices.clone()).await?;
                    }
                    _ => (),
                }
                println!();
            }
            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                change_info(addr, devices.clone(), property).await;
            }
            else => break
        }
    }

    Ok(())
}
