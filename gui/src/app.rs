use eframe::egui;
use eframe::epaint::{Color32, Fonts};
use egui::{FontData, FontDefinitions, FontFamily, Key, ScrollArea, Stroke, Ui};
use hex_brains_engine::simulation::{
    EngineCommand, EngineEvent, Hex, MutationConfig, SimulationConfig, Stats,
};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::fs;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use crate::drawing::draw_hexes;

pub struct CommandDispatcher {
    sender: Sender<EngineCommand>,
}

impl CommandDispatcher {
    fn send_create_snakes(&self, count: usize) {
        self.sender
            .send(EngineCommand::CreateSnakes(count))
            .unwrap();
    }

    fn send_update_simulation_config(&self, config: SimulationConfig) {
        self.sender
            .send(EngineCommand::UpdateSimulationConfig(config))
            .unwrap();
    }

    fn send_reset_world(&self) {
        self.sender.send(EngineCommand::ResetWorld).unwrap();
    }

    fn send_flip_running_state(&self) {
        self.sender.send(EngineCommand::FlipRunningState).unwrap();
    }

    fn send_decrease_speed(&self) {
        self.sender.send(EngineCommand::DecreaseSpeed).unwrap();
    }

    fn send_increase_speed(&self) {
        self.sender.send(EngineCommand::IncreaseSpeed).unwrap();
    }

    fn send_ignore_speed_limit(&self) {
        self.sender.send(EngineCommand::IgnoreSpeedLimit).unwrap();
    }

    fn send_advance_one_frame(&self) {
        self.sender.send(EngineCommand::AdvanceOneFrame).unwrap();
    }

    fn send_repaint_requested(&self) {
        self.sender.send(EngineCommand::RepaintRequested).unwrap();
    }
}

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

/// Configuration state and data management
pub struct ConfigState {
    pub config: super::Config,
    pub simulation_config: SimulationConfig,
    pub previous_simulation_config: SimulationConfig,
    pub stats: Stats,
    pub stats_history: VecDeque<(u32, Stats)>,
    pub history_limit: usize,
    pub smoothing_window: usize,
    pub hexes: Vec<Hex>,
    pub fonts: Fonts,
}

pub struct MyEguiApp {
    pub text: String,
    pub command_dispatcher: CommandDispatcher,
    pub engine_events_sender: Sender<EngineEvent>,
    pub engine_events_receiver: Receiver<EngineEvent>,
    pub engine_commands_receiver: Arc<Mutex<Receiver<EngineCommand>>>,
    pub ui_state: UiState,
    pub performance_stats: PerformanceStats,
    pub config_state: ConfigState,
}

impl MyEguiApp {
    fn load_config() -> SimulationConfig {
        let default_config = SimulationConfig {
            rows: 100,
            columns: 100,
            create_scents: false,
            scent_diffusion_rate: 0.2,
            scent_dispersion_per_step: 30.0,
            starting_snakes: 0,
            starting_food: 0,
            food_per_step: 2,
            plant_matter_per_segment: 100.0,
            wait_cost: 1.0,
            move_cost: 10.0,
            new_segment_cost: 100.0,
            size_to_split: 12,
            species_threshold: 0.2,
            add_walls: false,
            mutation: MutationConfig::default(),
            snake_max_age: 2_000,
            meat_energy_content: 5.0,
            plant_energy_content: 1.0,
        };
        if let Ok(contents) = fs::read_to_string("config.toml") {
            match toml::from_str(&contents) {
                Ok(loaded) => {
                    tracing::info!("Config loaded successfully from config.toml");
                    loaded
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config.toml: {}, using defaults", e);
                    default_config
                }
            }
        } else {
            tracing::info!("config.toml not found, using defaults");
            default_config
        }
    }

    fn save_config(&self) {
        if let Ok(toml_str) = toml::to_string(&self.config_state.simulation_config) {
            let _ = fs::write("config.toml", toml_str);
        }
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        engine_commands_sender: Sender<EngineCommand>,
        engine_events_sender: Sender<EngineEvent>,
        engine_events_receiver: Receiver<EngineEvent>,
        engine_commands_receiver: Receiver<EngineCommand>,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut font_definitions = FontDefinitions::default();
        font_definitions.font_data.insert(
            "firacode_nerd".to_owned(),
            FontData::from_static(include_bytes!("../../assets/FiraCodeNerdFont-Regular.ttf")),
        );
        font_definitions
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "firacode_nerd".to_owned());
        font_definitions
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "firacode_nerd".to_owned());
        cc.egui_ctx.set_fonts(font_definitions.clone());
        let simulation_config = Self::load_config();
        let config = super::Config {
            rows: simulation_config.rows,
            columns: simulation_config.columns,
            bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
            scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xAD, 0xD8, 0xE6, 50)),
            tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
            food_color: Stroke::new(1.0, Color32::YELLOW),
            add_walls: simulation_config.add_walls,
        };
        Self {
            text: String::new(),
            command_dispatcher: CommandDispatcher {
                sender: engine_commands_sender,
            },
            engine_events_sender,
            engine_events_receiver,
            engine_commands_receiver: Arc::new(Mutex::new(engine_commands_receiver)),
            ui_state: UiState {
                show_statistics: false,
                show_simulation_settings: false,
                show_mutation_settings: false,
                show_species: false,
                show_info: false,
                show_dna_settings: false,
                show_networks: false,
                selected_network: 0,
                started: false,
                paused: false,
            },
            performance_stats: PerformanceStats {
                total_frames: 0,
                last_frame: Instant::now(),
                updates_last_second: 0,
                last_second: Instant::now(),
                frames_last_second: 0,
                frames_per_second: 0,
                updates_per_second: 0,
                can_draw_frame: true,
            },
            config_state: ConfigState {
                config,
                simulation_config,
                previous_simulation_config: simulation_config,
                stats: Stats::default(),
                stats_history: VecDeque::new(),
                history_limit: super::HISTORY_LIMIT,
                smoothing_window: super::SMOOTHING_WINDOW,
                hexes: vec![],
                fonts: Fonts::new(1.0, 2 * 1024, font_definitions),
            },
        }
    }
    fn handle_events(&mut self, _ctx: &egui::Context) {
        if !self.ui_state.started {
            super::start_simulation(
                &self.engine_events_sender,
                Arc::clone(&self.engine_commands_receiver),
                _ctx.clone(),
                self.config_state.simulation_config,
            );
            self.ui_state.started = true;
            self.command_dispatcher
                .send_create_snakes(super::DEFAULT_SNAKES_TO_ADD);
        }
        self.engine_events_receiver
            .try_iter()
            .for_each(|result| match result {
                EngineEvent::SimulationFinished {
                    steps,
                    name,
                    duration,
                } => {
                    self.text.push_str(&format!(
                        "\nSimulation {name} finished in {steps} steps in {duration} ms"
                    ));
                }
                EngineEvent::FrameDrawn {
                    updates_left,
                    updates_done,
                } => {
                    self.text =
                        format!("{updates_left:.1} updates left, {updates_done} updates done");
                    self.performance_stats.can_draw_frame = true;
                    self.performance_stats.total_frames += 1;
                    self.performance_stats.updates_last_second += updates_done;
                    self.performance_stats.frames_last_second += 1;
                }
                EngineEvent::DrawData {
                    hexes,
                    stats,
                    frames,
                } => {
                    self.config_state.hexes = hexes;
                    self.config_state.stats = stats.clone();
                    self.config_state.stats_history.push_back((frames, stats));
                    if self.config_state.stats_history.len() > self.config_state.history_limit {
                        self.config_state.stats_history.pop_front();
                    }
                }
            });
        if self.performance_stats.last_second.elapsed().as_millis()
            > super::PERFORMANCE_UPDATE_INTERVAL_MS
        {
            self.performance_stats.last_second = Instant::now();
            self.performance_stats.updates_per_second = self.performance_stats.updates_last_second;
            self.performance_stats.frames_per_second = self.performance_stats.frames_last_second;
            self.performance_stats.updates_last_second = 0;
            self.performance_stats.frames_last_second = 0;
        }
    }

    fn render_windows(&mut self, ctx: &egui::Context) {
        super::windows::render_environment_settings_window(self, ctx);
        super::windows::render_mutation_settings_window(self, ctx);
        super::windows::render_dna_settings_window(self, ctx);
        super::windows::render_species_window(self, ctx);
        super::windows::render_statistics_window(self, ctx);
        super::windows::render_networks_window(self, ctx);
        super::windows::render_info_window(self, ctx);
    }

    fn render_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.menu_button("View", |ui| {
                ui.menu_button("Display Settings", |ui| {
                    ui.horizontal(|ui| {
                        egui::stroke_ui(
                            ui,
                            &mut self.config_state.config.bg_color,
                            "Background Color",
                        );
                        egui::stroke_ui(
                            ui,
                            &mut self.config_state.config.scent_color,
                            "Scent Color",
                        );
                        egui::stroke_ui(ui, &mut self.config_state.config.tail_color, "Tail Color");
                        egui::stroke_ui(ui, &mut self.config_state.config.food_color, "Food Color");
                    });
                });
            })
            .response
            .on_hover_text("Configure display settings");
            ui.menu_button("Help", |ui| {
                let checked = self.ui_state.show_info;
                if ui
                    .selectable_label(checked, if checked { "✓ Info" } else { "Info" })
                    .on_hover_text("Toggle help and keyboard shortcuts window (F1)")
                    .clicked()
                {
                    self.ui_state.show_info = !self.ui_state.show_info;
                }
            })
            .response
            .on_hover_text("Get help and information (F1)");
            if ui
                .button("🔄")
                .on_hover_text("Restart simulation (Ctrl+R)")
                .clicked()
            {
                self.command_dispatcher
                    .send_update_simulation_config(self.config_state.simulation_config);
                self.command_dispatcher.send_reset_world();
                self.command_dispatcher.send_create_snakes(10);
                self.ui_state.paused = false;
            }
            if ui
                .button("🌍")
                .on_hover_text("Toggle environment settings window (E)")
                .clicked()
            {
                self.ui_state.show_simulation_settings = !self.ui_state.show_simulation_settings;
            }
            if ui
                .button("")
                .on_hover_text("Toggle mutation settings window (M)")
                .clicked()
            {
                self.ui_state.show_mutation_settings = !self.ui_state.show_mutation_settings;
            }
            if ui
                .button("🧬")
                .on_hover_text("Toggle DNA settings window (D)")
                .clicked()
            {
                self.ui_state.show_dna_settings = !self.ui_state.show_dna_settings;
            }
            if ui
                .button("🐾")
                .on_hover_text("Toggle species window (P)")
                .clicked()
            {
                self.ui_state.show_species = !self.ui_state.show_species;
            }
            if ui
                .button("󰧑 ")
                .on_hover_text("Toggle neural networks window (N)")
                .clicked()
            {
                self.ui_state.show_networks = !self.ui_state.show_networks;
            }
            if ui
                .button("📊")
                .on_hover_text("Toggle statistics window (T)")
                .clicked()
            {
                self.ui_state.show_statistics = !self.ui_state.show_statistics;
            }
            // Add snakes
            if ui.button("🐍").on_hover_text("Add 10 snakes (S)").clicked() {
                self.command_dispatcher
                    .send_create_snakes(super::DEFAULT_SNAKES_TO_ADD);
            }
            // Play/Pause button
            let play_pause_icon = if self.ui_state.paused { "▶" } else { "⏸" };
            let play_button = egui::Button::new(play_pause_icon).fill(if self.ui_state.paused {
                Color32::from_rgb(0, 128, 0) // Dark green for better contrast
            } else {
                Color32::from_rgb(128, 0, 0) // Dark red for better contrast
            });
            if ui
                .add(play_button)
                .on_hover_text(if self.ui_state.paused {
                    "Play simulation (Space)"
                } else {
                    "Pause simulation (Space)"
                })
                .clicked()
            {
                self.ui_state.paused = !self.ui_state.paused;
                self.command_dispatcher.send_flip_running_state();
            }
            if ui
                .small_button("-")
                .on_hover_text("Decrease speed (-)")
                .clicked()
            {
                self.command_dispatcher.send_decrease_speed();
            }
            if ui
                .small_button("+")
                .on_hover_text("Increase speed (+)")
                .clicked()
            {
                self.command_dispatcher.send_increase_speed();
            }
            ui.label(format!(
                "x{:.1}",
                self.performance_stats.updates_per_second as f32
                    / self.performance_stats.frames_per_second as f32
            ));
        });
    }

    fn render_central_panel(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        draw_hexes(ui, &self.config_state.hexes, &self.config_state.config);
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.label(&self.text);
            });
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::PlusEquals)) {
            self.command_dispatcher.send_increase_speed();
        }
        if ctx.input(|i| i.key_pressed(Key::Minus)) {
            self.command_dispatcher.send_decrease_speed();
        }
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.command_dispatcher.send_ignore_speed_limit();
        }
        if ctx.input(|i| i.key_pressed(Key::O)) {
            self.command_dispatcher.send_create_snakes(1);
        }
        if ctx.input(|i| i.key_pressed(Key::A)) {
            self.command_dispatcher.send_advance_one_frame();
        }
        // New shortcuts for toolbar buttons
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.ui_state.show_info = !self.ui_state.show_info;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::R)) {
            self.command_dispatcher
                .send_update_simulation_config(self.config_state.simulation_config);
            self.command_dispatcher.send_reset_world();
            self.command_dispatcher
                .send_create_snakes(super::DEFAULT_SNAKES_TO_ADD);
            self.ui_state.paused = false;
        }
        if ctx.input(|i| i.key_pressed(Key::E)) {
            self.ui_state.show_simulation_settings = !self.ui_state.show_simulation_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::M)) {
            self.ui_state.show_mutation_settings = !self.ui_state.show_mutation_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::D)) {
            self.ui_state.show_dna_settings = !self.ui_state.show_dna_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            self.ui_state.show_species = !self.ui_state.show_species;
        }
        if ctx.input(|i| i.key_pressed(Key::N)) {
            self.ui_state.show_networks = !self.ui_state.show_networks;
        }
        if ctx.input(|i| i.key_pressed(Key::T)) {
            self.ui_state.show_statistics = !self.ui_state.show_statistics;
        }
        if ctx.input(|i| i.key_pressed(Key::S)) {
            self.command_dispatcher
                .send_create_snakes(super::DEFAULT_SNAKES_TO_ADD);
        }
        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.ui_state.paused = !self.ui_state.paused;
            self.command_dispatcher.send_flip_running_state();
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::profile_scope!("gui::update");
        if puffin::are_scopes_on() {
            puffin_egui::profiler_window(ctx);
            puffin::GlobalProfiler::lock().new_frame();
        }
        self.handle_events(ctx);
        self.render_windows(ctx);
        if self.config_state.simulation_config != self.config_state.previous_simulation_config {
            self.save_config();
            if !super::only_star_fields_differ(
                &self.config_state.simulation_config,
                &self.config_state.previous_simulation_config,
            ) || self.ui_state.paused
            {
                self.command_dispatcher
                    .send_update_simulation_config(self.config_state.simulation_config);
            }
            self.config_state.previous_simulation_config = self.config_state.simulation_config;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_toolbar(ui);
            self.render_central_panel(ctx, ui);
        });
        self.handle_keyboard_shortcuts(ctx);
        if self.performance_stats.can_draw_frame {
            ctx.request_repaint();
            self.performance_stats.can_draw_frame = false;
        }
        self.performance_stats.last_frame = Instant::now();
        self.command_dispatcher.send_repaint_requested();
    }
}
