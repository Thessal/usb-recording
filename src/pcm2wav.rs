use std::fs::File;
use std::io::{self, Read, Write};
use crate::state;

use rusb::{DeviceHandle, Direction, Recipient, RequestType, GlobalContext, UsbContext};
use anyhow::{Context, Result};
use chrono::Local;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tokio::process::Command;


// --- AUDIO SPECIFICATION CONSTANTS ---
// Matches the user's request: pcm_s16le, 48000 Hz, stereo, s16
const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2; // Stereo
const BITS_PER_SAMPLE: u16 = 16; // s16le

// Calculate dependent constants
const AUDIO_FORMAT: u16 = 1; // 1 = PCM
const BLOCK_ALIGN: u16 = CHANNELS * (BITS_PER_SAMPLE / 8); // 2 channels * 2 bytes/sample = 4 bytes
const BYTE_RATE: u32 = SAMPLE_RATE * CHANNELS as u32 * (BITS_PER_SAMPLE / 8) as u32; // 48000 * 2 * 2 = 192000

// --- Hourly Processing ---

pub async fn postprocessing(state_arc: &Arc<Mutex<state::SystemState>>) -> Result<()> {
    println!("Running hourly processing...");
    
    let active_file = {
        let s = state_arc.lock().unwrap();
        let guard = s.current_filename.lock().unwrap();
        guard.clone()
    };

    // Changed glob to look for .raw files
    // let entries = glob::glob("*.raw").context("Glob error")?;
    println!("processing (not really)");

    //for entry in entries {
    //    if let Ok(path) = entry {
    //        let raw_str = path.to_string_lossy().to_string();
    //        let txt_str = format!("{}.txt", raw_str);

    //        if let Some(ref active) = active_file {
    //            if active == &raw_str { continue; }
    //        }

    //        if !Path::new(&txt_str).exists() {
    //            println!("Processing: {}", raw_str);
    //            let _ = Command::new(SCRIPT_PATH)
    //                .arg(&raw_str)
    //                .arg(&txt_str)
    //                .spawn(); 
    //        }
    //    }
    //}
    Ok(())
}


fn convert() -> io::Result<()> {
    // --- FILE PATHS ---
    const INPUT_FILE: &str = "input.pcm";
    const OUTPUT_FILE: &str = "output.wav";

    println!("Attempting to convert '{}' to '{}'...", INPUT_FILE, OUTPUT_FILE);
    println!("Target Specification: {}Hz, {} channels, {} bits/sample (s16le)", SAMPLE_RATE, CHANNELS, BITS_PER_SAMPLE);

    // 1. READ THE RAW PCM DATA
    let mut input_file = match File::open(INPUT_FILE) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: Could not open input file '{}'. Make sure it exists.", INPUT_FILE);
            eprintln!("Details: {}", e);
            return Err(e);
        }
    };
    
    let mut pcm_data = Vec::new();
    input_file.read_to_end(&mut pcm_data)?;

    let data_size = pcm_data.len() as u32;

    if data_size == 0 {
        println!("Input file is empty. WAV file creation aborted.");
        return Ok(());
    }

    // RIFF Chunk Size: 36 bytes (header minus ChunkID and ChunkSize) + data_size
    let chunk_size = 36 + data_size;
    let subchunk2_size = data_size;

    // 2. CREATE THE WAV HEADER
    let mut header = Vec::with_capacity(44);

    // RIFF Chunk
    header.extend_from_slice(b"RIFF");                     // ChunkID
    header.extend_from_slice(&chunk_size.to_le_bytes());   // ChunkSize (36 + Data Size)
    header.extend_from_slice(b"WAVE");                     // Format

    // fmt Subchunk
    header.extend_from_slice(b"fmt ");                     // Subchunk1ID
    header.extend_from_slice(&(16u32).to_le_bytes());      // Subchunk1Size (16 for PCM)
    header.extend_from_slice(&AUDIO_FORMAT.to_le_bytes()); // AudioFormat (1 = PCM)
    header.extend_from_slice(&CHANNELS.to_le_bytes());     // NumChannels (2)
    header.extend_from_slice(&SAMPLE_RATE.to_le_bytes());  // SampleRate (48000)
    header.extend_from_slice(&BYTE_RATE.to_le_bytes());    // ByteRate (192000)
    header.extend_from_slice(&BLOCK_ALIGN.to_le_bytes());  // BlockAlign (4)
    header.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes()); // BitsPerSample (16)

    // data Subchunk
    header.extend_from_slice(b"data");                     // Subchunk2ID
    header.extend_from_slice(&subchunk2_size.to_le_bytes()); // Subchunk2Size (Data Size)

    // 3. WRITE HEADER AND DATA TO OUTPUT FILE
    let mut output_file = File::create(OUTPUT_FILE)?;

    // Write the 44-byte WAV header
    output_file.write_all(&header)?;

    // Write the raw PCM audio data
    output_file.write_all(&pcm_data)?;

    println!("\nConversion successful!");
    println!("Data Size: {} bytes", data_size);
    println!("Output File: '{}' created with 44-byte WAV header.", OUTPUT_FILE);

    Ok(())
}
