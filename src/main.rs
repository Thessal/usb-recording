mod audio;
mod pcm2wav;
mod usbctrl;
mod state;

use rusb::{DeviceHandle, Direction, Recipient, RequestType, GlobalContext, UsbContext};
use anyhow::{Context, Result};
use chrono::Local;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tokio::process::Command;

// --- Configuration ---

// Recording and Postprocessing Configuration
const POSTPROC_PERIOD: u64 = 3600;
const MIN_SPEECH_LENGTH: u64 = 10;


#[tokio::main]
async fn main() -> Result<()> {
    let state = Arc::new(Mutex::new(state::SystemState {
        is_recording: Arc::new(AtomicBool::new(false)),
        current_filename: Arc::new(Mutex::new(None)),
        stop_timer_start: None,
    }));

    // Turn off LED
    let _ = usbctrl::turn_off_led().context("Failed to turn off the LED");

    // 1. Hourly Processor (Background)
    let processor_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(POSTPROC_PERIOD)).await;
            if let Err(e) = pcm2wav::postprocessing(&processor_state).await {
                eprintln!("Error in hourly processor: {}", e);
            }
        }
    });

    println!("Starting Raw PCM Monitor...");
    // let usb_device = open_usb_device(VID, PID).context("Failed to open USB device")?;
    let usb_device: DeviceHandle<rusb::GlobalContext> = usbctrl::open_device();

    // 2. Main Monitoring Loop
    loop {
        let val = usbctrl::read_voice_status(&usb_device);
        let mut s = state.lock().unwrap();
        let recording_active = s.is_recording.load(Ordering::SeqCst);

        if val != 0 {
            // === Signal Active ===
            if !recording_active {
                audio::start_audio_thread(&mut s);
            } else {
                if s.stop_timer_start.is_some() {
                    println!("Signal returned. Resetting stop timer.");
                    s.stop_timer_start = None;
                }
            }
        } else {
            // === Signal Inactive ===
            if recording_active {
                match s.stop_timer_start {
                    None => s.stop_timer_start = Some(Instant::now()),
                    Some(start_time) => {
                        if start_time.elapsed() >= Duration::from_secs(MIN_SPEECH_LENGTH) {
                            println!("Silence detected ({}s). Stopping recording.", MIN_SPEECH_LENGTH);
                            s.is_recording.store(false, Ordering::SeqCst);
                            
                            {
                                let mut filename_guard = s.current_filename.lock().unwrap();
                                *filename_guard = None;
                            }
                            s.stop_timer_start = None;
                        }
                    }
                }
            }
        }

        drop(s);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
