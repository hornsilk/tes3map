use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_center_from_cell, get_tri_at_cell, CellKey};
use voronoice::*;
use egui::Pos2;

use crate::generate_random_color;


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

    let n = interventions.keys().len();
    let mut centers: Vec<Point> = Vec::with_capacity(n as usize);
    let mut shapes: Vec<Shape> = Vec::new();
    let mut colors: Vec<Color32> = Vec::with_capacity(n as usize);

    for key in interventions.keys() {
        let center_pos2 = get_center_from_cell(dimensions, to_screen, key.clone());
        centers.push( Point{x: (center_pos2.x as f64) , y: (center_pos2.y as f64) });

        let mut color = Color32::from_gray(0);
        if let Some(region_name) = &interventions[key].region {
            // generate a random string hashed by "region_name_x_y"
            let color_from_hash = generate_random_color(&format!("{}_{}_{}",region_name,key.0,key.1));
            color = Color32::from_rgb(
                color_from_hash.0,
                color_from_hash.1,
                color_from_hash.2,
            );
        }

        color = color.gamma_multiply(0.1);
        colors.push(color);
    }

    let my_voronoi = VoronoiBuilder::default()
        .set_sites(centers)
        .set_bounding_box(bounding_box) 
        .build()
        .unwrap();


    for (my_cell, color) in my_voronoi.iter_cells().zip(colors) {
        let verts = my_cell.iter_vertices().collect::<Vec<&Point>>();
        let n_vertecies = verts.len();
        let mut verts_pos2: Vec<Pos2> = Vec::with_capacity(n_vertecies as usize);
    
        for vert in verts {
            verts_pos2.push(Pos2::new(vert.x as f32, vert.y as f32));
        }


        let polygon = Shape::convex_polygon(verts_pos2, color, Stroke::new(1.0, Color32::from_rgb(0, 0, 0)));
        
        shapes.push(polygon);
        
        
        // let my_new_center = Pos2::new(my_cell.site_position().x as f32, my_cell.site_position().y as f32);
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