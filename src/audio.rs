use crate::state;

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

// Audio Configuration
const AUDIO_DEVICE: &str = "plughw:ArrayUAC10,0";
const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u32 = 2; 
const SCRIPT_PATH: &str = "./script.sh";



// --- Raw Audio Recording Logic ---

pub fn start_audio_thread(state: &mut state::SystemState) {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    // Changed extension to .raw to indicate headerless PCM
    let filename = format!("{}.raw", timestamp);
    
    let run_flag = Arc::new(AtomicBool::new(true));
    state.is_recording = run_flag.clone();
    state.stop_timer_start = None;
    
    {
        let mut fn_guard = state.current_filename.lock().unwrap();
        *fn_guard = Some(filename.clone());
    }

    println!("Starting raw recording: {}", filename);

    thread::spawn(move || {
        if let Err(e) = record_raw_loop(&filename, run_flag) {
            eprintln!("Recording Error: {}", e);
        } else {
            println!("Recording finished: {}", filename);
        }
    });
}

pub fn record_raw_loop(filename: &str, run_flag: Arc<AtomicBool>) -> Result<()> {
    use alsa::{Direction, ValueOr};
    use alsa::pcm::{PCM, HwParams, Format, Access};

    // 1. Open ALSA
    let pcm = PCM::new(AUDIO_DEVICE, Direction::Capture, false)
        .context(format!("Failed to open ALSA device '{}'", AUDIO_DEVICE))?;

    // 2. Hardware Params
    let hwp = HwParams::any(&pcm)?;
    hwp.set_channels(CHANNELS)?;
    hwp.set_rate(SAMPLE_RATE, ValueOr::Nearest)?;
    hwp.set_format(Format::s16())?; // s16le is standard
    hwp.set_access(Access::RWInterleaved)?;
    pcm.hw_params(&hwp)?;

    // 3. Open File with Buffer
    let file = File::create(filename).context("Failed to create output file")?;
    // Buffer size of 64KB significantly reduces syscall overhead
    let mut writer = BufWriter::with_capacity(64 * 1024, file);

    // 4. Create IO Handle and Data Buffer
    // 'io' is the interface to the stream
    let io = pcm.io_i16()?;

    // We need a buffer to read INTO.
    // Size = frames * channels.
    // 1024 frames (period size) is standard latency for this use case.
    const FRAMES: usize = 1024;
    let mut buf = [0i16; FRAMES * CHANNELS as usize];

    pcm.start()?;

    while run_flag.load(Ordering::SeqCst) {
        // readi takes a mutable slice and returns the number of FRAMES read
        match io.readi(&mut buf) {
            Ok(frames_read) => {
                if frames_read > 0 {
                    // Convert frames to total samples (e.g., 1 frame = 2 samples in stereo)
                    let samples_count = frames_read * CHANNELS as usize;

                    // Iterate only over the data we actually read
                    for &sample in &buf[..samples_count] {
                        // Write s16 as little-endian bytes
                        writer.write_all(&sample.to_le_bytes())?;
                    }
                }
            }
            Err(e) => {
                // Try to recover from buffer overruns (XRUN)
                // If recovery fails, we log and exit the loop
                if let Err(recover_err) = pcm.try_recover(e, false) {
                    eprintln!("ALSA Critical Error: {}", recover_err);
                    break;
                }
            }
        }
    }
    
    writer.flush()?; // Ensure all data is written to disk
    pcm.drop()?; 
    
    Ok(())
}
