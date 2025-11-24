use std::sync::{
    atomic::AtomicBool,
    Arc, Mutex,
};
use std::time::Instant;

// --- State Management ---
pub struct SystemState {
    pub is_recording: Arc<AtomicBool>,
    pub current_filename: Arc<Mutex<Option<String>>>,
    pub unprocessed_files: Arc<Mutex<Vec<String>>>,
    pub stop_timer_start: Option<Instant>,
    pub modeldir: String,
    pub datadir: String,
    pub language: String,
}
