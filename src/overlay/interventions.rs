use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, get_tri_at_cell, CellKey};
use voronoice::*;
use egui::Pos2;
// use std::cmp::max;



pub fn create_voronoi_polygons(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let n_points = interventions.keys().len();
    let mut centers: Vec<Point> = Vec::with_capacity(n_points as usize);
    // let mut shapes: Vec<Shape> = Vec::with_capacity(n_points as usize);
    let mut shapes: Vec<Shape> = Vec::new();


    for key in interventions.keys() {
        // let center = get_center_from_cell(dimensions, to_screen, key.clone());
        // centers.push( Point{x: center.x as f64, y:center.y as f64});
        centers.push( Point{x: (key.0 - dimensions.min_x) as f64, y: (key.1 - dimensions.min_y) as f64});
    }
    println!("start verts");
    for center in centers.clone() {
        println!("{},{}",center.x, center.y);
    }
    println!("end verts");
    // let temp = to_screen * Pos2{x:(dimensions.max_x - dimensions.min_x) as f32 , y:(dimensions.max_y - dimensions.min_y) as f32};
    // let bounding_size = (2.0*(max(temp.x.ceil() as i32, temp.y.ceil() as i32) as f64));
    // let bounding_size = 50.0; //TODO calc this size somehow
    let bounding_size = 100.0; //TODO calc this size somehow

    let to_screen_voronoi = to_screen.clone();
    // let to_screen_voronoi =  RectTransform {to:to_screen.to()., from:(to_screen.from()*2)};
    println!("x:[{},{}], y:[{},{}]", dimensions.min_x,dimensions.max_x, dimensions.min_y, dimensions.max_y);

    // builds a voronoi diagram from the set of sites above, bounded by a square of size 4
    // let sites = vec![
    //  Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 0.0 }, Point { x: 0.0, y: 1.0 }
    // ];


    let my_voronoi = VoronoiBuilder::default()
        .set_sites(centers)
        .set_bounding_box(BoundingBox::new_centered_square(bounding_size)) 
        .set_lloyd_relaxation_iterations(5)
        .build()
        .unwrap();

    // println!("Second cell has site {:?}, voronoi vertices {:?} and delaunay triangles {:?}",
    //     my_cell.site_position(),
    //     my_cell.iter_vertices().collect::<Vec<&Point>>(),
    //     my_cell.triangles().iter().collect::<Vec<&usize>>());
    
    
    // let my_cell = my_voronoi.cell(10);
    for my_cell in my_voronoi.iter_cells() {

        let my_verts = my_cell.iter_vertices().collect::<Vec<&Point>>();
        let n_vertecies = my_verts.len();
        let mut my_pos2s: Vec<Pos2> = Vec::with_capacity(n_vertecies as usize);
    
        for vert in my_verts {
            // let my_pos2 = Pos2{x: (vert.x as f32 + dimensions.min_x as f32), y: (vert.y as f32 + dimensions.min_y as f32)};
            let my_pos2 = Pos2{x: (vert.x as f32 - dimensions.min_x as f32), y: (vert.y as f32 - dimensions.min_y as f32)};
            // let my_pos2 = Pos2{x: (vert.x as f32 ), y: (vert.y as f32 )};
            my_pos2s.push(to_screen_voronoi*my_pos2);
        }
        let color = Color32::from_rgb(0, 255, 0);
        let color2 = Color32::from_rgb(0, 255, 0);
        let color2 = color2.gamma_multiply(0.0);
        let my_polygon = Shape::convex_polygon(my_pos2s, color2, Stroke::new(3.0, color));
    
        shapes.push(my_polygon);
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