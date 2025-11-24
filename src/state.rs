use std::sync::{
    atomic::AtomicBool,
    Arc, Mutex,
};
use std::time::Instant;

// --- State Management ---
pub struct SystemState {
    pub is_recording: Arc<AtomicBool>,
    pub current_filename: Arc<Mutex<Option<String>>>,
    pub stop_timer_start: Option<Instant>,
}
