use gt_tauri::sim_thread::SimThread;

/// Tauri managed state wrapping the background simulation thread.
pub struct SimState {
    pub sim: SimThread,
}

impl SimState {
    pub fn new() -> Self {
        Self {
            sim: SimThread::new(),
        }
    }
}
