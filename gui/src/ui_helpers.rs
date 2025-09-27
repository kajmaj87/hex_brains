use eframe::egui;
use eframe::emath;
use egui::Ui;

pub fn add_drag_value<T: emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    speed: f64,
    tooltip: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(tooltip);
        ui.add(egui::DragValue::new(value).speed(speed))
            .on_hover_text(tooltip);
    });
}

pub fn add_checkbox(ui: &mut Ui, label: &str, value: &mut bool, tooltip: &str) {
    ui.add(egui::Checkbox::new(value, label))
        .on_hover_text(tooltip);
}
