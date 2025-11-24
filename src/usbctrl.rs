use rusb::{DeviceHandle, Direction, Recipient, RequestType, GlobalContext};
use std::time::Duration;
use anyhow::{Context, Result};

// USB Control Transfer Parameters
const VID: u16 = 0x2886; // Vendor ID
const PID: u16 = 0x0018; // Product ID
const TIMEOUT: Duration = Duration::from_millis(8000);

// Data Read config
const USB_REQ_TYPE_READ: u8 = rusb::constants::LIBUSB_REQUEST_TYPE_VENDOR | rusb::constants::LIBUSB_ENDPOINT_IN;
const USB_REQUEST: u8 = 0;
const USB_WVALUE_SPEECH: u16 = 0x40 | 0x80 | 0x16; // Speech recognition 
const USB_WINDEX_SPEECH: u16 = 19;
const USB_LENGTH: usize = 8;

// Data Write config
const USB_REQ_TYPE_WRITE: u8 = rusb::request_type(
    Direction::Out,
    RequestType::Vendor,
    Recipient::Device,
);
const USB_WVALUE_LED: u16 = 0x20;
const USB_WVALUE_VADLED: u16 = 0x22;
const USB_WINDEX_WRITE: u16 = 0x1C;

pub fn open_device() -> DeviceHandle<rusb::GlobalContext> {
    let handle: Option<DeviceHandle<rusb::GlobalContext>> = rusb::open_device_with_vid_pid(VID, PID);
    match handle {
        Some(x) => x,
        None => panic!("Error: Device not found")
    }
}

pub fn read_voice_status(handle: &DeviceHandle<GlobalContext>) -> u8 {
    let mut buf = [0u8; USB_LENGTH];
    match handle.read_control(
        USB_REQ_TYPE_READ, 
        USB_REQUEST, 
        USB_WVALUE_SPEECH, 
        USB_WINDEX_SPEECH, 
        &mut buf, 
        TIMEOUT
    ) {
        Ok(_) => buf[0],
        Err(e) => {
            eprintln!("USB Read Error: {}", e);
            0
        }
    }
}

pub fn turn_off_led() -> Result<()> {
    println!("Looking for device {:04x}:{:04x} to turn off LED...", VID, PID);

    // Open the device
    let handle: DeviceHandle<rusb::GlobalContext> = open_device();

    let _ = handle.claim_interface(0).context("Failed to claim interface");

    let data = [0u8]; //set brightness to zero.

    println!("Setting LED brightness to zero (0x20)...");
    let _bytes_written = handle.write_control(
        USB_REQ_TYPE_WRITE, // bmRequestType
        USB_REQUEST,            // bRequest
        USB_WVALUE_LED,         // wValue
        USB_WINDEX_WRITE,         // wIndex
        &data,        // data buffer
        TIMEOUT,      // timeout
    )?;

    println!("Turning off VAD LED (0x22)...");
    let _bytes_written_2 = handle.write_control(
        USB_REQ_TYPE_WRITE,
        USB_REQUEST,
        USB_WVALUE_VADLED,
        USB_WINDEX_WRITE,
        &data,
        TIMEOUT,
    )?;

    let _ = handle.release_interface(0).context("Failed to release interface");

    println!("LED turned off.");

    Ok(())
}
