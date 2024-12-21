use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_tri_at_cell, CellKey};
use voronoice::*;
use egui::Pos2;
use std::cmp::max;


pub fn convert_from_voronoi(
    p: &Point,
    dimensions: &Dimensions, 
) -> Pos2 {
   
    let new_x = p.x as i32 - dimensions.min_x;
    let new_y = -p.y as i32 + dimensions.max_y;
    let new_center = Pos2{x:new_x as f32, y:new_y as f32};

    new_center

}

pub fn create_voronoi_polygons(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let dx = dimensions.max_x - dimensions.min_x;
    let dy = dimensions.max_y - dimensions.min_y;
    let bounding_size = max(dx, dy) as f64 * 1.2;

    let n_points = interventions.keys().len();
    let mut centers: Vec<Point> = Vec::with_capacity(n_points as usize);
    let mut shapes: Vec<Shape> = Vec::new();

    for key in interventions.keys() {
        centers.push( Point{x: (key.0 as f64) , y: (key.1 as f64) });
    }
    let my_voronoi = VoronoiBuilder::default()
        .set_sites(centers)
        .set_bounding_box(BoundingBox::new_centered_square(bounding_size)) 
        .build()
        .unwrap();


    for my_cell in my_voronoi.iter_cells() {
        let my_verts = my_cell.iter_vertices().collect::<Vec<&Point>>();
        let n_vertecies = my_verts.len();
        let mut my_verticies: Vec<Pos2> = Vec::with_capacity(n_vertecies as usize);
    
        for vert in my_verts {
            let my_vert = convert_from_voronoi(vert, dimensions);
            my_verticies.push(to_screen*my_vert);
        }
        let color = Color32::from_rgb(0, 255, 0);
        let color2 = Color32::from_rgb(0, 255, 0);
        let color2 = color2.gamma_multiply(0.1);
        let my_polygon = Shape::convex_polygon(my_verticies, color2, Stroke::new(3.0, color));
        
        
        shapes.push(my_polygon);
        
        
        let my_new_center = convert_from_voronoi(my_cell.site_position(),dimensions);
        
        let radius = 10.0;
        let my_dot = Shape::circle_filled(to_screen*my_new_center, radius, color);
        shapes.push(my_dot);
    }

    shapes

}


pub fn get_intervention_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let color = Color32::from_rgb(180, 25, 25);
    let color2 = Color32::from_rgb(180, 25, 25);
    let color2 = color2.gamma_multiply(0.0);

    // let shapes_len =
    //     (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::new();
    
    for key in interventions.keys() {
        let tri = get_tri_at_cell(dimensions, to_screen, key.clone());
        let shape = Shape::convex_polygon(tri, color2, Stroke::new(3.0, color));
        shapes.push(shape);
    }

    let voronoi_cells = create_voronoi_polygons(to_screen, dimensions, interventions);
    shapes.extend(voronoi_cells);

    shapes
}