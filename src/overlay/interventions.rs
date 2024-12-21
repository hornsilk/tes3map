use std::{collections::HashMap};

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_center_from_cell, get_tri_at_cell, CellKey};
use voronoice::*;
use egui::Pos2;




pub fn create_voronoi_polygons(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let dx = (to_screen.to().max.x - to_screen.to().min.x) as f64;
    let cx = ((to_screen.to().max.x + to_screen.to().min.x)/2.0) as f64;
    let dy = (to_screen.to().max.y - to_screen.to().min.y) as f64;
    let cy = ((to_screen.to().max.y + to_screen.to().min.y)/2.0) as f64;
    let bounding_box = BoundingBox::new( Point{x:cx,y:cy}, dx, dy);

    let n_points = interventions.keys().len();
    let mut centers: Vec<Point> = Vec::with_capacity(n_points as usize);
    let mut shapes: Vec<Shape> = Vec::new();

    for key in interventions.keys() {
        let temp = get_center_from_cell(dimensions, to_screen, key.clone());
        centers.push( Point{x: (temp.x as f64) , y: (temp.y as f64) });
    }

    let my_voronoi = VoronoiBuilder::default()
        .set_sites(centers)
        .set_bounding_box(bounding_box) 
        .build()
        .unwrap();


    for my_cell in my_voronoi.iter_cells() {
        let verts = my_cell.iter_vertices().collect::<Vec<&Point>>();
        let n_vertecies = verts.len();
        let mut verts_pos2: Vec<Pos2> = Vec::with_capacity(n_vertecies as usize);
    
        for vert in verts {
            verts_pos2.push(Pos2 {x: vert.x as f32, y:vert.y as f32});
        }


        let color = Color32::from_rgb(0, 255, 0);
        let color2 = Color32::from_rgb(0, 255, 0);
        let color2 = color2.gamma_multiply(0.1);
        let polygon = Shape::convex_polygon(verts_pos2, color2, Stroke::new(3.0, color));
        
        shapes.push(polygon);
        
        
        // let my_new_center = Pos2 {x: my_cell.site_position().x as f32, y:my_cell.site_position().y as f32};
        // let radius = 10.0;
        // let my_dot = Shape::circle_filled(my_new_center, radius, color);
        // shapes.push(my_dot);
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