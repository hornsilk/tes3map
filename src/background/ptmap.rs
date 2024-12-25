use std::collections::HashMap;

use egui::{Color32, ColorImage, Rect, Pos2};

use crate::Dimensions;

static GRID: usize = 9;

use serde::Deserialize;
use image::{ImageError, Rgba};


static PNG_FILES: &[(&str, &[u8])] = &[
    ("tr_map", include_bytes!("pt_maps/tr_map.png")),
    ("pc_map", include_bytes!("pt_maps/pc_map.png")),
];

#[derive(Debug, Deserialize)]
struct Map {
    name: String,
    grid_pxls: i32,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    file: String,
}

#[derive(Debug, Deserialize)]
struct MapsData {
    maps: Vec<Map>,
}


pub fn load_image_as_color_image(data: &[u8]) -> Result<ColorImage, ImageError> {
    let img = image::load_from_memory(data)?;
    let rgba_image = img.to_rgba8();

    let (width, height) = rgba_image.dimensions();
    let pixels = rgba_image
        .pixels()
        .map(|&Rgba([r, g, b, a])| Color32::from_rgba_unmultiplied(r, g, b, a))
        .collect();

    Ok(ColorImage {
        size: [width as usize, height as usize],
        pixels,
    })
}

pub fn generate_ptmap(
    dimensions: &Dimensions,
) -> ColorImage {
    
    // calculate map size
    let height = dimensions.pixel_height(GRID);
    let width = dimensions.pixel_width(GRID);
    // let size = height * width;
    // let mut pixels: Vec<Color32> = Vec::with_capacity(size);
    
    
    
    // Include maps metadata
    let file_content = include_str!("pt_maps/maps_metadata.json");
    let maps_metadata: MapsData = serde_json::from_str(file_content).expect("Missing Metadata File");


    // load map images
    let mut img_hash: HashMap<String, ColorImage> = HashMap::new();
    for (file_name, file_contents) in PNG_FILES {
        let img_data = load_image_as_color_image(file_contents).expect("Missing Image File");
        let img_name = file_name.to_owned().to_owned();
        img_hash.insert(img_name, img_data);
    }

    // Load the image from the embedded bytes

    let map_type = "Morrowind";
    let mut image_data: ColorImage = ColorImage::new([height, width],Color32::from_gray(0));

    for map_metadata in maps_metadata.maps {
        if map_metadata.name == map_type {
            
            let _maxx = map_metadata.max_x;
            let _maxy = map_metadata.max_y;
            let _miny = map_metadata.min_y;
            // println!("dimensions: ({},{}) to ({},{})",dimensions.min_x,dimensions.min_y,dimensions.max_x,dimensions.max_y);
            // println!("grid: ({},{}) to ({},{})",map_metadata.min_x,map_metadata.min_y,map_metadata.max_x,map_metadata.max_y);
            let full_image_data = img_hash[&map_metadata.file].clone();

            let pixels_per_point = map_metadata.grid_pxls as f32;
            let min = Pos2::new(
                (dimensions.min_x - map_metadata.min_x) as f32, 
                (map_metadata.max_y - dimensions.max_y) as f32
            );
            let max = Pos2::new(
                (dimensions.max_x - map_metadata.min_x + 1) as f32, 
                (map_metadata.max_y - dimensions.min_y + 1) as f32, 
            );
            // println!("min: ({},{}) -- max: ({},{})",min.x,min.y,max.x,max.y);
            let region = Rect::from_min_max(min, max);
            image_data = full_image_data.region(&region, Some(pixels_per_point));

        }
    }


    


    // Print the parsed data
    // for map in data.maps {
    //     println!("Map Name: {}", map.name);
    //     println!("Grid Pixels: {}", map.grid_pxls);
    //     println!("X Range: {} to {}", map.x_min, map.x_max);
    //     println!("Y Range: {} to {}", map.y_min, map.y_max);
    //     println!("File: {}", map.file);
    //     println!("---");
    // }


    // for grid_y in 0..height {
    //     for grid_x in (0..width).rev() {
    //         // we can divide by grid to get the cell and subtract the bounds to get the cell coordinates
    //         let x = (grid_x / GRID) as i32 + dimensions.min_x;
    //         let y = (grid_y / GRID) as i32 + dimensions.min_y;

    //         // get LAND record
    //         let key = (x, y);
    //         if let Some(land) = landscape_records.get(&key) {
    //             // get remainder
    //             let hx = grid_x % GRID;
    //             let hy = grid_y % GRID;

    //             let heightmap = land.world_map_data.data.clone().to_vec();
    //             pixels.push(get_map_color(heightmap[hy][hx] as f32));
    //         } else {
    //             pixels.push(Color32::TRANSPARENT);
    //         }
    //     }
    // }

    // pixels.reverse();

    // ColorImage {
    //     pixels,
    //     size: [width, height],
    // }
    image_data
}
