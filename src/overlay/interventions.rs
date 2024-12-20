use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Rect, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_tri_at_cell, get_rect_at_cell, CellKey};
use voronoice::*;
use egui::Pos2;
// use std::cmp::max;



pub fn create_voronoi_polygons(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    println!("Start Voronoi");
    let n_points = interventions.keys().len();
    let mut centers: Vec<Point> = Vec::with_capacity(n_points as usize);
    // let mut shapes: Vec<Shape> = Vec::with_capacity(n_points as usize);
    let mut shapes: Vec<Shape> = Vec::new();


    let dx = (dimensions.max_x - dimensions.min_x) as f64;
    let dy = (dimensions.max_y - dimensions.min_y) as f64;
    let bounding_size = 4.0; //TODO calc this size somehow

    // println!("start verts");
    for key in interventions.keys() {
        // println!("{},{}",key.0, key.1);
        centers.push( Point{x: (key.0 as f64) / dx, y: (key.1 as f64) / dy});
    }
    for center in centers.clone() {
        println!("{},{}",center.x, center.y);
    }
    // println!("end verts");

    let my_voronoi = VoronoiBuilder::default()
        .set_sites(centers)
        .set_bounding_box(BoundingBox::new_centered_square(bounding_size)) 
        .set_lloyd_relaxation_iterations(5)
        .build()
        .unwrap();

    for my_cell in my_voronoi.iter_cells() {
        // let my_verts = my_cell.iter_vertices().collect::<Vec<&Point>>();
        // let n_vertecies = my_verts.len();
        // let mut my_pos2s: Vec<Pos2> = Vec::with_capacity(n_vertecies as usize);
    
        // for vert in my_verts {
        //     // let my_pos2 = Pos2{x: (vert.x as f32 + dimensions.min_x as f32), y: (vert.y as f32 + dimensions.min_y as f32)};
        //     // let my_pos2 = Pos2{x: (vert.x as f32 - dimensions.min_x as f32), y: (vert.y as f32 - dimensions.min_y as f32)};
        //     // let my_pos2 = Pos2{x: (vert.x as f32 ), y: (vert.y as f32 )};
        //     let my_pos2 = Pos2{x: (vert.x as f32 ), y: (-vert.y as f32 )};
        //     my_pos2s.push(to_screen_voronoi*my_pos2);
        // }
        let color = Color32::from_rgb(0, 255, 0);
        // let color2 = Color32::from_rgb(0, 255, 0);
        // let color2 = color2.gamma_multiply(0.0);
        // let my_polygon = Shape::convex_polygon(my_pos2s, color2, Stroke::new(3.0, color));
    
        
        // shapes.push(my_polygon);
        
        let temp_point = my_cell.site_position();  
        let temp_shift_x = ((temp_point.x)*dx/bounding_size) as i32;
        let temp_shift_y = ((temp_point.y)*dy/bounding_size) as i32;
        let new_x = temp_shift_x - dimensions.min_x;
        let new_y = temp_shift_y - dimensions.min_y;
        let my_new_center = Pos2{x:new_x as f32, y:new_y as f32};
        
        let radius = 10.0;
        let my_dot = Shape::circle_filled(to_screen*my_new_center, radius, color);
        shapes.push(my_dot);
    }


    // my_voronoi.iter_cells().for_each(|cell| {
    //     println!("Vertices of cell: {:?}", cell.iter_vertices().collect::<Vec<&Point>>())
    // });
    

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