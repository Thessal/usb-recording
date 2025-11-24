mod pcmdump;
mod pcm2wav;
mod usbctrl;
mod state;

use rusb::DeviceHandle;
use anyhow::{Context, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Location of ggml-large-v3.bin
    #[arg(short, long, default_value = "/home/jongkook90/models")]
    modeldir: String,

    /// Location of data 
    #[arg(short, long, default_value = "/home/jongkook90/recordings")]
    datadir: String,

    /// Location of data 
    #[arg(short, long, default_value = "ko")]
    lang: String,

    /// Period to run speech recognition
    #[arg(short, long, default_value_t = 3600)]
    proc_period: u64,

    /// Length of silence needed to stop recording
    #[arg(short, long, default_value_t = 10)]
    segment_length: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(state::SystemState {
        is_recording: Arc::new(AtomicBool::new(false)),
        current_filename: Arc::new(Mutex::new(None)),
        stop_timer_start: None,
        unprocessed_files: Arc::new(Mutex::new(vec![])),
        modeldir: args.modeldir,
        language: args.lang,
        datadir: args.datadir,
    }));

    // Turn off LED
    let _ = usbctrl::turn_off_led().context("Failed to turn off the LED");

    // 1. Hourly Processor (Background)
    let processor_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(args.proc_period)).await;
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
                pcmdump::start_audio_thread(&mut s);
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
                        if start_time.elapsed() >= Duration::from_secs(args.segment_length) {
                            println!("Silence detected ({}s). Stopping recording.", args.segment_length);
                            s.is_recording.store(false, Ordering::SeqCst);
                            
                            {
                                let mut filename_guard = s.current_filename.lock().unwrap();
                                s.unprocessed_files.lock().unwrap().push(filename_guard.clone().unwrap()); // add the file to process
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
