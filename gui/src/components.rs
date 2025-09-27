use egui::Ui;

pub fn render_vision_ranges(
    ui: &mut Ui,
    front: &mut u32,
    left: &mut u32,
    right: &mut u32,
    name: &str,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{name} Front range"))
            .on_hover_text("Vision range directly ahead");
        ui.add(egui::DragValue::new(front).speed(1.0))
            .on_hover_text(format!(
                "Set front vision distance for {}",
                name.to_lowercase()
            ));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{name} Left range"))
            .on_hover_text("Vision range to the left");
        ui.add(egui::DragValue::new(left).speed(1.0))
            .on_hover_text(format!(
                "Set left vision distance for {}",
                name.to_lowercase()
            ));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{name} Right range"))
            .on_hover_text("Vision range to the right");
        ui.add(egui::DragValue::new(right).speed(1.0))
            .on_hover_text(format!(
                "Set right vision distance for {}",
                name.to_lowercase()
            ));
    });
}
