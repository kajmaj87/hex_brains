use std::time::Instant;

/// UI state tracking window visibility and user interface state
pub struct UiState {
    pub show_statistics: bool,
    pub show_simulation_settings: bool,
    pub show_mutation_settings: bool,
    pub show_species: bool,
    pub show_info: bool,
    pub show_dna_settings: bool,
    pub show_networks: bool,
    pub selected_network: u32,
    pub started: bool,
    pub paused: bool,
}

/// Performance tracking and frame rate statistics
pub struct PerformanceStats {
    pub total_frames: usize,
    pub last_frame: Instant,
    pub updates_last_second: u32,
    pub last_second: Instant,
    pub frames_last_second: u32,
    pub frames_per_second: u32,
    pub updates_per_second: u32,
    pub can_draw_frame: bool,
}
