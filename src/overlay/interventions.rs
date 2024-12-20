use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Rounding, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_rect_at_cell, CellKey};



pub fn get_intervention_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let color = Color32::from_rgb(255, 0, 0);
    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);
    
    for key in interventions.keys() {
        let rect = get_rect_at_cell(dimensions, to_screen, key.clone());
        let shape = Shape::rect_stroke(rect, Rounding::default(), Stroke::new(4.0, color));
        shapes.push(shape);
    }
    shapes
}