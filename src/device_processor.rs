use bluer::{Device, AddressType, Modalias};
use uuid::Uuid;
use anyhow::Result;
use std::collections::{HashSet, HashMap};

pub struct BluetoothData {
    name: Option<String>,
    address_type: AddressType,
    icon: Option<String>,
    appearance: Option<u16>,
    uuids: Option<HashSet<Uuid>>,
    paired: bool,
    connected: bool,
    trusted: bool,
    blocked: bool,
    wake_allowed: bool,
    alias: String,
    legacy_pairing: bool,
    modalias: Option<Modalias>,
    rssi: Option<i16>,
    tx_power: Option<i16>,
    manufacturer_data: Option<HashMap<u16, Vec<u8>>>,
    service_data: Option<HashMap<Uuid, Vec<u8>>>,
    services_resolved: bool,
    advertising_flags: Vec<u8>,
    advertising_data: HashMap<u8, Vec<u8>>
}

pub async fn device_to_data(device: &Device) -> Result<BluetoothData> {
    Ok(BluetoothData {
        name: device.name().await?,
        address_type: device.address_type().await?,
        icon: device.icon().await?,
        appearance: device.appearance().await?,
        uuids: device.uuids().await?,
        paired: device.is_paired().await?,
        connected: device.is_connected().await?,
        trusted: device.is_trusted().await?,
        blocked: device.is_blocked().await?,
        wake_allowed: device.is_wake_allowed().await?,
        alias: device.alias().await?,
        legacy_pairing: device.is_legacy_pairing().await?,
        modalias: device.modalias().await?,
        rssi: device.rssi().await?,
        tx_power: device.tx_power().await?,
        manufacturer_data: device.manufacturer_data().await?,
        service_data: device.service_data().await?,
        services_resolved: device.is_services_resolved().await?,
        advertising_flags: device.advertising_flags().await?,
        advertising_data: device.advertising_data().await?
    })
}