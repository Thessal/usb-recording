use std::fs::{self, File};
use std::io::{self, Read, Write};
use crate::state;

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::process::{Command};
use std::process::{Stdio};

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
    println!("Running hourly postprocessing...");
    
    let (modeldir, language, files, active_file) = {
        let s = state_arc.lock().unwrap();
        let guard_current = s.current_filename.lock().unwrap();
        let guard_files = s.unprocessed_files.lock().unwrap();
        let modeldir = s.modeldir.clone();
        let language = s.language.clone();
        (modeldir, language, guard_files.clone(), guard_current.clone())
    };

    for file in files.iter(){
        println!("Processing Files : {}", file);
        let mut outfilename = String::from(file);
        (0..4).for_each(|_| { outfilename.pop(); });
        let filename_wav = outfilename.clone() + ".wav";
        let filename_txt = outfilename.clone() + ".txt";
        let filename_log = outfilename.clone() + ".log";
        let convert_result = convert(file, filename_wav.as_str()).context("PCM to WAV conversion failed.");
        let _: Result<()> = match convert_result{
            Ok(()) => speech_recognition(
                filename_wav.as_str(), filename_txt.as_str(), filename_log.as_str(),
                modeldir.as_str(), language.as_str()
            ).await,
            Err(e) => Err(e)
        };
    }

    println!("Files currently recording : {:?}", active_file);

    {
        let s = state_arc.lock().unwrap();
        let mut guard_files = s.unprocessed_files.lock().unwrap();
        guard_files.drain(0..files.len());
    }
    println!("Hourly postprocessing finished.");

    Ok(())
}

async fn speech_recognition(input_filename: &str, output_filename: &str, log_filename: &str, modeldir: &str, language: &str) -> Result<()>{
    let proc = Command::new("whisper-cli")
            .arg("-l")
            .arg(language)
            .arg("-m")
            .arg(format!("{}/ggml-large-v3.bin", modeldir))
            .arg(input_filename)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
    match proc{
        Ok(child) => {
            match child.wait_with_output().await {
                Ok(output) => {
                    fs::write(log_filename, &output.stderr).expect("Failed to write logfile.");
                    fs::write(output_filename, &output.stdout).expect("Failed to write recognition result.");

                    if !output.status.success() {
                        eprintln!("\nWarning: 'postprocessing' command exited with a non-zero status.");
                    }

                    Ok(())
                },
                Err(e) => {
                    eprintln!("Error: Failed to wait for or collect output from child process.");
                    Err(e.into())
                }
            }
        },
        Err(e) => {
            eprintln!("Error: Failed to run whisper-cli. Binary, model, or file is missing. ({})", input_filename);
            eprintln!("Details: {}", e);
            return Err(e.into());
        }
    }
}

fn convert(input_filename: &str, output_filename: &str) -> io::Result<()> {

    println!("Attempting to convert '{}' to '{}'...", input_filename, output_filename);
    println!("Target Specification: {}Hz, {} channels, {} bits/sample (s16le)", SAMPLE_RATE, CHANNELS, BITS_PER_SAMPLE);

    // 1. READ THE RAW PCM DATA
    let mut input_file = match File::open(input_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: Could not open input file '{}'. Make sure it exists.", input_filename);
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
    let mut output_file = File::create(output_filename)?;

    // Write the 44-byte WAV header
    output_file.write_all(&header)?;

    // Write the raw PCM audio data
    output_file.write_all(&pcm_data)?;

    println!("\nConversion successful!");
    println!("Data Size: {} bytes", data_size);
    println!("Output File: '{}' created with 44-byte WAV header.", output_filename);

    output_file.sync_all()?;
    Ok(())
}
