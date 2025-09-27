use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::drawing::u32_to_color;
use crate::ui_helpers::{add_checkbox, add_drag_value};
use crate::{components, MyEguiApp};

pub fn render_environment_settings_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Environment Settings")
        .open(&mut app.ui_state.show_simulation_settings)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Size*")
                    .on_hover_text("Grid size in hexes (width and height)");
                ui.add(egui::DragValue::new(&mut app.config_state.config.columns).speed(1.0))
                    .on_hover_text("Adjust grid dimensions");
                app.config_state.config.rows = app.config_state.config.columns;
                app.config_state.simulation_config.rows = app.config_state.config.rows;
                app.config_state.simulation_config.columns = app.config_state.config.columns;
            });
            add_checkbox(
                ui,
                "Add walls*",
                &mut app.config_state.config.add_walls,
                "Add walls around the grid perimeter",
            );
            add_drag_value(
                ui,
                "Food per step",
                &mut app.config_state.simulation_config.food_per_step,
                1.0,
                "Number of food items added each simulation step",
            );
            add_drag_value(
                ui,
                "Energy per segment",
                &mut app.config_state.simulation_config.plant_matter_per_segment,
                1.0,
                "Energy content of plant food per snake segment",
            );
            add_drag_value(
                ui,
                "Wait cost",
                &mut app.config_state.simulation_config.wait_cost,
                1.0,
                "Energy cost for waiting action",
            );
            add_drag_value(
                ui,
                "Move cost",
                &mut app.config_state.simulation_config.move_cost,
                1.0,
                "Energy cost for moving action",
            );
            add_drag_value(
                ui,
                "New segment energy cost",
                &mut app.config_state.simulation_config.new_segment_cost,
                1.0,
                "Energy cost to grow a new segment",
            );
            add_drag_value(
                ui,
                "Size to split",
                &mut app.config_state.simulation_config.size_to_split,
                1.0,
                "Minimum segments required to split/reproduce",
            );
            add_drag_value(
                ui,
                "Aging starts at",
                &mut app.config_state.simulation_config.snake_max_age,
                1.0,
                "Age when snakes start losing energy",
            );
            add_drag_value(
                ui,
                "Species coloring threshold",
                &mut app.config_state.simulation_config.species_threshold,
                1.0,
                "Genetic distance for species clustering",
            );
            add_checkbox(
                ui,
                "Create smell (low performance, memory leaks)",
                &mut app.config_state.simulation_config.create_scents,
                "Enable scent diffusion (experimental, may cause performance issues)",
            );
            add_drag_value(
                ui,
                "Smell diffusion rate",
                &mut app.config_state.simulation_config.scent_diffusion_rate,
                1.0,
                "Rate at which scents spread",
            );
            add_drag_value(
                ui,
                "Smell dispersion rate per step",
                &mut app.config_state.simulation_config.scent_dispersion_per_step,
                1.0,
                "Scent dispersion per simulation step",
            );
            ui.label("Settings marked with * will only take effect after a restart.");
        });
}

pub fn render_mutation_settings_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Mutation Settings")
        .open(&mut app.ui_state.show_mutation_settings)
        .show(ctx, |ui| {
            ui.label("Senses:")
                .on_hover_text("Configure sensory capabilities that can mutate");
            add_checkbox(
                ui,
                "Chaos gene",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .chaos_input_enabled,
                "Allow random input to neural networks",
            );
            add_checkbox(
                ui,
                "Food smelling",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .scent_sensing_enabled,
                "Enable scent-based food detection",
            );
            add_checkbox(
                ui,
                "Plant vision",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .plant_vision_enabled,
                "Allow vision of plant food",
            );
            if app
                .config_state
                .simulation_config
                .mutation
                .plant_vision_enabled
            {
                components::render_vision_ranges(
                    ui,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .plant_vision_front_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .plant_vision_left_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .plant_vision_right_range,
                    "Plant",
                );
            }
            add_checkbox(
                ui,
                "Meat vision",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .meat_vision_enabled,
                "Allow vision of meat food",
            );
            if app
                .config_state
                .simulation_config
                .mutation
                .meat_vision_enabled
            {
                components::render_vision_ranges(
                    ui,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .meat_vision_front_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .meat_vision_left_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .meat_vision_right_range,
                    "Meat",
                );
            }
            add_checkbox(
                ui,
                "Obstacle vision",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .obstacle_vision_enabled,
                "Allow vision of obstacles/walls",
            );
            if app
                .config_state
                .simulation_config
                .mutation
                .obstacle_vision_enabled
            {
                components::render_vision_ranges(
                    ui,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .obstacle_vision_front_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .obstacle_vision_left_range,
                    &mut app
                        .config_state
                        .simulation_config
                        .mutation
                        .obstacle_vision_right_range,
                    "Obstacle",
                );
            }
            ui.label("Mutation settings:")
                .on_hover_text("Configure neural network mutation parameters");
            add_drag_value(
                ui,
                "Weights perturbation chance",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .weight_perturbation_chance,
                1.0,
                "Probability of randomly adjusting connection weights",
            );
            add_drag_value(
                ui,
                "Weights perturbation range",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .weight_perturbation_range,
                1.0,
                "Maximum adjustment amount for weights",
            );
            add_checkbox(
                ui,
                "Perturb disabled connections",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .perturb_disabled_connections,
                "Allow mutation of disabled neural connections",
            );
            add_drag_value(
                ui,
                "Weights reset chance",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .weight_reset_chance,
                1.0,
                "Probability of resetting weights to new random values",
            );
            add_drag_value(
                ui,
                "Weights reset range",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .weight_reset_range,
                1.0,
                "Range for new random weights",
            );
            add_checkbox(
                ui,
                "Perturb reset connections",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .perturb_disabled_connections,
                "Allow perturbation of newly reset connections",
            );
            add_drag_value(
                ui,
                "Connection flip chance",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .connection_flip_chance,
                1.0,
                "Probability of enabling/disabling connections",
            );
            add_drag_value(
                ui,
                "Dna mutation chance",
                &mut app
                    .config_state
                    .simulation_config
                    .mutation
                    .dna_mutation_chance,
                1.0,
                "Probability of mutating snake DNA segments",
            );
        });
}

pub fn render_dna_settings_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("DNA Settings")
        .open(&mut app.ui_state.show_dna_settings)
        .show(ctx, |ui| {
            ui.label("Disable segments from possible genes during mutation:")
                .on_hover_text("Uncheck to allow this segment type in DNA mutations");
            let mut segment_configs = [
                (
                    "Muscle",
                    &mut app.config_state.simulation_config.mutation.disable_muscle,
                    egui::Color32::LIGHT_RED,
                ),
                (
                    "Solid",
                    &mut app.config_state.simulation_config.mutation.disable_solid,
                    egui::Color32::BROWN,
                ),
                (
                    "Solar",
                    &mut app.config_state.simulation_config.mutation.disable_solar,
                    egui::Color32::LIGHT_BLUE,
                ),
                (
                    "Stomach",
                    &mut app.config_state.simulation_config.mutation.disable_stomach,
                    egui::Color32::LIGHT_GREEN,
                ),
            ];
            for (i, (name, disable, color)) in segment_configs.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.colored_label(*color, format!("{i}: {name}"));
                    ui.checkbox(disable, "Disable");
                });
            }
        });
}

pub fn render_species_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Species")
        .open(&mut app.ui_state.show_species)
        .show(ctx, |_ui| {});
}

pub fn render_statistics_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Statistics")
        .open(&mut app.ui_state.show_statistics)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("History limit");
                ui.add(egui::DragValue::new(&mut app.config_state.history_limit).speed(10.0));
            });
            ui.horizontal(|ui| {
                ui.label("Smoothing window");
                ui.add(egui::DragValue::new(&mut app.config_state.smoothing_window).speed(10.0));
            });
            let current_frame = app
                .config_state
                .stats_history
                .back()
                .map(|(f, _)| *f as f64)
                .unwrap_or(0.0);
            let raw_plant_energy: Vec<(f64, f64)> = app
                .config_state
                .stats_history
                .iter()
                .map(|(f, s)| {
                    (
                        *f as f64 - current_frame,
                        s.total_plant_energy as f64 / 1000.0,
                    )
                })
                .collect();
            let raw_meat_energy: Vec<(f64, f64)> = app
                .config_state
                .stats_history
                .iter()
                .map(|(f, s)| {
                    (
                        *f as f64 - current_frame,
                        s.total_meat_energy as f64 / 1000.0,
                    )
                })
                .collect();
            let raw_snake_energy: Vec<(f64, f64)> = app
                .config_state
                .stats_history
                .iter()
                .map(|(f, s)| {
                    (
                        *f as f64 - current_frame,
                        s.total_snake_energy as f64 / 1000.0,
                    )
                })
                .collect();
            let plant_energy: PlotPoints = raw_plant_energy
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let start = if i >= app.config_state.smoothing_window {
                        i - app.config_state.smoothing_window + 1
                    } else {
                        0
                    };
                    let sum: f64 = raw_plant_energy[start..=i].iter().map(|(_, y)| *y).sum();
                    let count = (i - start + 1) as f64;
                    [raw_plant_energy[i].0, sum / count]
                })
                .collect();
            let meat_energy: PlotPoints = raw_meat_energy
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let start = if i >= app.config_state.smoothing_window {
                        i - app.config_state.smoothing_window + 1
                    } else {
                        0
                    };
                    let sum: f64 = raw_meat_energy[start..=i].iter().map(|(_, y)| *y).sum();
                    let count = (i - start + 1) as f64;
                    [raw_meat_energy[i].0, sum / count]
                })
                .collect();
            let snake_energy: PlotPoints = raw_snake_energy
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let start = if i >= app.config_state.smoothing_window {
                        i - app.config_state.smoothing_window + 1
                    } else {
                        0
                    };
                    let sum: f64 = raw_snake_energy[start..=i].iter().map(|(_, y)| *y).sum();
                    let count = (i - start + 1) as f64;
                    [raw_snake_energy[i].0, sum / count]
                })
                .collect();
            let plant_line = Line::new(plant_energy).name("Plant Energy (/1000)");
            let meat_line = Line::new(meat_energy).name("Meat Energy (/1000)");
            let snake_line = Line::new(snake_energy).name("Snake Energy (/1000)");
            Plot::new("stats_plot")
                .view_aspect(2.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(plant_line);
                    plot_ui.line(meat_line);
                    plot_ui.line(snake_line);
                });
            if app.config_state.stats.species.species.is_empty() {
                ui.label("No species yet.");
                return;
            }
            let mut sorted = app.config_state.stats.species.species.clone();
            sorted.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
            let bars: Vec<Bar> = sorted
                .iter()
                .enumerate()
                .map(|(i, specie)| {
                    Bar::new(i as f64, specie.members.len() as f64)
                        .name(format!("{}", specie.id))
                        .fill(u32_to_color(specie.id))
                })
                .collect();
            let bar_chart = BarChart::new(bars);
            Plot::new("species_plot")
                .view_aspect(2.0)
                .show(ui, |plot_ui| {
                    plot_ui.bar_chart(bar_chart);
                });
        });
}

pub fn render_networks_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Networks").open(&mut app.ui_state.show_networks).show(ctx, |ui| {
        let specie_ids = &app.config_state.stats.species.species.iter().map(|specie| specie.id).collect::<Vec<u32>>();
        if specie_ids.is_empty() {
            ui.label("No networks yet").on_hover_text("No species have formed yet - start a simulation to see neural networks");
            return;
        }
        let selected_specie_in_list = specie_ids.contains(&app.ui_state.selected_network);
        if !selected_specie_in_list {
            app.ui_state.selected_network = specie_ids[0];
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Specie")
                .selected_text(format!("{:?}", app.ui_state.selected_network))
                .show_ui(ui, |ui| {
                    for specie_id in specie_ids {
                        ui.selectable_value(&mut app.ui_state.selected_network, *specie_id, format!("{specie_id:?}"));
                    }
                }).response.on_hover_text("Select species to view its neural network");
            if ui.button("Next").on_hover_text("View next species").clicked() {
                app.ui_state.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == app.ui_state.selected_network).unwrap() + 1) % specie_ids.len()];
            }
            if ui.button("Previous").on_hover_text("View previous species").clicked() {
                app.ui_state.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == app.ui_state.selected_network).unwrap() + specie_ids.len() - 1) % specie_ids.len()];
            }
        });
        ui.collapsing("Information", |ui| {
            ui.label("Green connections mean that the weight is positive, red connections mean that the weight is negative. The thicker the connection, the higher the weight.").on_hover_text("Connection visualization guide");
            ui.label("Positive weight means the snake wants to do the given action if it encounters this sensory input.").on_hover_text("Weight interpretation");
            ui.label("Bias is a constant value of 1.0, chaos is a random number from range 0.0 .. 1.0 generated each tick").on_hover_text("Special input explanations");
            ui.label("Network cost is the energy it takes each turn to 'think'").on_hover_text("Neural network energy cost");

            ui.horizontal(|ui| {
                ui.label(
                    r#"Input Nodes:
                    "#).on_hover_text("List of neural network inputs");
                ui.label(
                    r#"Output Nodes
                    Move Forward
                    Move Left
                    Move Right
                    Wait"#).on_hover_text("List of neural network outputs");
            });
        }).header_response.on_hover_text("Show/hide neural network information");
        if let Some(selected_specie) = app.config_state.stats.species.species.iter().find(|specie| specie.id == app.ui_state.selected_network) {
            ui.label(format!("Network run cost: {}", selected_specie.leader_network.run_cost())).on_hover_text("Energy cost per neural network evaluation");
            crate::drawing::draw_neural_network(ui, &app.config_state.fonts, selected_specie.id, &selected_specie.leader_network.get_nodes(), &selected_specie.leader_network.get_active_connections());
        }
    });
}

pub fn render_info_window(app: &mut MyEguiApp, ctx: &egui::Context) {
    egui::Window::new("Info")
        .open(&mut app.ui_state.show_info)
        .show(ctx, |ui| {
            ui.label("Press 'o' to add one snake")
                .on_hover_text("Keyboard shortcut to spawn a single snake");
            ui.label("Press 'a' to stop simulation and advance one frame (useful for debug)")
                .on_hover_text("Debug shortcut: pause and step one frame");
            ui.label("Press '+' to increase speed")
                .on_hover_text("Speed up simulation playback");
            ui.label("Press '-' to decrease speed")
                .on_hover_text("Slow down simulation playback");
            ui.label("Press 'tab' to ignore speed limit")
                .on_hover_text("Run simulation as fast as possible");
            ui.label("All enabled settings take effect immediately")
                .on_hover_text("Changes apply without restarting");
            ui.label("To change disabled settings, stop the simulation first")
                .on_hover_text("Some settings require simulation restart");
            ui.horizontal(|ui| {
                ui.label(format!("Tot: {}", app.performance_stats.total_frames))
                    .on_hover_text(format!(
                        "Total frames: {}",
                        app.performance_stats.total_frames
                    ));
                ui.label(format!(
                    "FPS: {:.1}",
                    app.performance_stats.frames_per_second
                ))
                .on_hover_text(format!(
                    "Frames per second: {:.1}",
                    app.performance_stats.frames_per_second
                ));
                ui.label(format!("UPS: {}", app.performance_stats.updates_per_second))
                    .on_hover_text(format!(
                        "Updates per second: {}",
                        app.performance_stats.updates_per_second
                    ));
                ui.label(format!(
                    "Spd: x{:.1}",
                    app.performance_stats.updates_per_second as f32
                        / app.performance_stats.frames_per_second as f32
                ))
                .on_hover_text(format!(
                    "Speed: x{:.1}",
                    app.performance_stats.updates_per_second as f32
                        / app.performance_stats.frames_per_second as f32
                ));
                ui.label(format!("Old: {}", app.config_state.stats.oldest_snake))
                    .on_hover_text(format!(
                        "Oldest snake: {}",
                        app.config_state.stats.oldest_snake
                    ));
                ui.label(format!("Gen: {}", app.config_state.stats.max_generation))
                    .on_hover_text(format!(
                        "Max generation: {}",
                        app.config_state.stats.max_generation
                    ));
                ui.label(format!("Mut: {}", app.config_state.stats.max_mutations))
                    .on_hover_text(format!(
                        "Max mutations: {}",
                        app.config_state.stats.max_mutations
                    ));
                ui.label(format!(
                    "Snk: {}/{}",
                    app.config_state.stats.total_snakes, app.config_state.stats.total_segments
                ))
                .on_hover_text(format!(
                    "Snakes/segments: {}/{}",
                    app.config_state.stats.total_snakes, app.config_state.stats.total_segments
                ));
                ui.label(format!("Food: {}", app.config_state.stats.total_food))
                    .on_hover_text(format!("Food: {}", app.config_state.stats.total_food));
                ui.label(format!(
                    "Spc: {}",
                    app.config_state.stats.species.species.len()
                ))
                .on_hover_text(format!(
                    "Species: {}",
                    app.config_state.stats.species.species.len()
                ));
                ui.label(format!("Snt: {}", app.config_state.stats.total_scents))
                    .on_hover_text(format!("Scents: {}", app.config_state.stats.total_scents));
                ui.label(format!("Ent: {}", app.config_state.stats.total_entities))
                    .on_hover_text(format!(
                        "Entities: {}",
                        app.config_state.stats.total_entities
                    ));
                ui.label(format!(
                    "P/M: {}/{}",
                    app.config_state.stats.total_plants, app.config_state.stats.total_meat
                ))
                .on_hover_text(format!(
                    "Plants/Meat: {}/{}",
                    app.config_state.stats.total_plants, app.config_state.stats.total_meat
                ));
                ui.label(format!(
                    "Stm: P/M {}/{}",
                    app.config_state.stats.total_plants_in_stomachs,
                    app.config_state.stats.total_meat_in_stomachs
                ))
                .on_hover_text(format!(
                    "Stomachs: P/M {}/{}",
                    app.config_state.stats.total_plants_in_stomachs,
                    app.config_state.stats.total_meat_in_stomachs
                ));
                ui.label(format!(
                    "SnkE: {}",
                    app.config_state.stats.total_snake_energy
                ))
                .on_hover_text(format!(
                    "Total snake energy: {}",
                    app.config_state.stats.total_snake_energy
                ));
                ui.label(format!("TotE: {}", app.config_state.stats.total_energy))
                    .on_hover_text(format!(
                        "Total energy: {}",
                        app.config_state.stats.total_energy
                    ));
            });
        });
}
