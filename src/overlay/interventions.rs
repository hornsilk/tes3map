use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_tri_at_cell, CellKey};



pub fn get_intervention_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let color = Color32::from_rgb(180, 25, 25);
    let color2 = Color32::from_rgb(180, 25, 25);
    let color2 = color2.gamma_multiply(0.0);
    
    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);
    
    for key in interventions.keys() {
        let tri = get_tri_at_cell(dimensions, to_screen, key.clone());
        let shape = Shape::convex_polygon(tri, color2, Stroke::new(3.0, color));
        shapes.push(shape);
    }
    shapes
}