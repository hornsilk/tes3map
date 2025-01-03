use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Shape};
use tes3::esp::Cell;

use crate::{
    dimensions::Dimensions, CellKey,
    get_center_from_cell, get_long_tri_at_cell, get_nonagon_at_cell, get_rect_at_cell, get_kyne_bird_at_cell,
    break_ties_todd_howard_spiral, rect_to_edges
};
use voronoice::*;
use egui::{Pos2, Rounding};

use std::cmp::max;

use crate::generate_random_color;


pub fn create_kingsstep_polygons(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    interventions: &HashMap<CellKey, Cell>,
    cell_records: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let n = interventions.keys().len();
    let mut shapes: Vec<Shape> = Vec::new();
    if n < 1 {
        return shapes
    }

    let mut centers: Vec<(i32,i32)> = Vec::with_capacity(n as usize);
    let mut colors: Vec<Color32> = Vec::with_capacity(n as usize);
    let mut edge_lists: Vec<Vec<Shape>> = Vec::with_capacity(n as usize);
    let mask_color: Color32 = Color32::from_gray(0).gamma_multiply(0.8);

    for key in interventions.keys() {
        centers.push(key.clone());

        let cell_name = &interventions[key].name; 
        // generate a random string hashed by "cell_name_(x,y)"
        let color_from_hash = generate_random_color(&format!("{}_({},{})",cell_name,key.0,key.1));
        let color = Color32::from_rgb(
            color_from_hash.0,
            color_from_hash.1,
            color_from_hash.2,
        );
        colors.push(color);
        edge_lists.push(Vec::new());
    }

    for x in dimensions.min_x..dimensions.max_x+1 {
        for y in dimensions.min_y..dimensions.max_y+1{

            let key = (x,y);
            let cell_has_region = cell_records
                .get(&key)
                .and_then(|value| value.region.as_ref()) 
                .map_or(false, |region| !region.is_empty()); 


            if cell_has_region {
                let mut min_idx = 0;
                if centers.len() > 1 {
                    // find closest node
                    let mut dist_map: HashMap<i32, (i32, (i32,i32))> = HashMap::with_capacity(n as usize);
                    for (i, (cx, cy)) in centers.clone().into_iter().enumerate() {
                        let dx = (cx - x).abs();
                        let dy = (cy - y).abs();
                        let kings_dist = max(dx, dy);
    
                        dist_map.insert(i as i32, (kings_dist, (cx,cy))); // keep (cx, cy), the node location, for breaking ties
                    }
                                
                    let mut min_val = dist_map[&min_idx].0;
                    for i in 1..(n as i32) {
                        if dist_map[&i].0 < min_val {
                            min_idx = i;
                            min_val = dist_map[&min_idx].0;
                        }
                        else if dist_map[&i].0 == min_val {
                            let node_a = dist_map[&min_idx].1;
                            let node_b = dist_map[&i].1;
    
                            if break_ties_todd_howard_spiral((x,y),node_a,node_b) {
                                min_idx = i;
                                min_val = dist_map[&min_idx].0;
                            }
                        }
                    }
                }
                let color = colors[min_idx as usize].gamma_multiply(0.2);

                let rect = get_rect_at_cell(dimensions, to_screen, (x,y));
                let shape = Shape::rect_filled(rect, Rounding::default(), color);
                shapes.push(shape);

                let edges = rect_to_edges(rect);
                edge_lists[min_idx as usize].extend(edges);
            }
            else {
                let rect = get_rect_at_cell(dimensions, to_screen, (x,y));
                let shape = Shape::rect_filled(rect, Rounding::default(), mask_color);
                shapes.push(shape)
            }
        }
    }

    for edge_list in edge_lists {
        let mut unique_edges: Vec<Shape> = Vec::new();
        // println!("New Edge List");
        for edge in edge_list {
            if unique_edges.contains(&edge) {
                let index = unique_edges.iter().position(|r| *r == edge).unwrap();
                let _ = unique_edges.swap_remove(index);
                // println!("    remove to {}", unique_edges.len());
            }
            else {
                unique_edges.push(edge);
                // println!("    add to {}", unique_edges.len());
            }
        }
        shapes.extend(unique_edges);
    }

    shapes
}


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
            // generate a random string hashed by "region_name_(x,y)"
            let color_from_hash = generate_random_color(&format!("{}_({},{})",region_name,key.0,key.1));
            color = Color32::from_rgb(
                color_from_hash.0,
                color_from_hash.1,
                color_from_hash.2,
            );
        }

        color = color.gamma_multiply(0.2);
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
    cell_records: &HashMap<CellKey, Cell>,
    icon_type: &str,
    intervention_engine: &str,
) -> Vec<Shape> {
    let color = Color32::from_rgb(0, 0, 0);
    let mut fill_color = Color32::from_rgb(0, 0, 0);
    
    let mut shapes: Vec<Shape> = Vec::new();
        
    if intervention_engine == "Pythagorean" {
        if interventions.len() > 1 {
            let voronoi_cells = create_voronoi_polygons(to_screen, dimensions, interventions);
            shapes.extend(voronoi_cells);
        }
    } else {
        let kings_step_cells = create_kingsstep_polygons(to_screen, dimensions, interventions, cell_records);
        shapes.extend(kings_step_cells);
    }

    for key in interventions.keys() {
        let radius = 3.0;
        let center = get_center_from_cell(dimensions, to_screen, key.clone());
        let mut shape = Shape::circle_filled(center, radius, fill_color);

        
        if icon_type == "almsivi" {
            let tri = get_long_tri_at_cell(dimensions, to_screen, key.clone());
            fill_color = Color32::from_rgb(180, 25, 25);
            shape = Shape::convex_polygon(tri, fill_color, Stroke::new(1.5, color));
        }
        else if icon_type == "divine" {
            fill_color = Color32::from_gray(200); //.gamma_multiply(0.0);
            
            let nonagon = get_nonagon_at_cell(dimensions, to_screen, key.clone());
            shape = Shape::convex_polygon(nonagon, fill_color, Stroke::new(1.5, color));
        }
        else if  icon_type == "kyne" {
            fill_color = Color32::from_rgb(0, 100, 0); //.gamma_multiply(0.0);
            
            let nonagon = get_kyne_bird_at_cell(dimensions, to_screen, key.clone());
            shape = Shape::convex_polygon(nonagon, fill_color, Stroke::new(1.5, color));
            
        }

        shapes.push(shape);
    }


    shapes
}