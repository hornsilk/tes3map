use std::collections::HashMap;

use eframe::epaint::{Color32, ColorImage};
use tes3::esp::{Landscape, LandscapeFlags};

use crate::{
    CellKey, depth_to_color, Dimensions, DimensionsZ, height_to_color, SavedUiData, VERTEX_CNT,
};

fn get_color_for_height(value: f32, dimensions: DimensionsZ, ui_data: SavedUiData) -> Color32 {
    if value < dimensions.min_z {
        return Color32::TRANSPARENT;
    }

    if value < 0.0 {
        depth_to_color(value, dimensions, ui_data)
    } else {
        height_to_color(value, dimensions, ui_data)
    }
}
pub fn generate_heightmap(
    pixels: &[f32],
    size: [usize; 2],
    dimensions_z: DimensionsZ,
    ui_data: SavedUiData,
) -> ColorImage {
    let mut img = ColorImage::new(size, Color32::WHITE);
    let p = pixels
        .iter()
        .map(|f| get_color_for_height(*f, dimensions_z, ui_data))
        .collect::<Vec<_>>();
    img.pixels = p;
    img
}

pub fn calculate_heights(
    landscape_records: &HashMap<CellKey, Landscape>,
    dimensions: &Dimensions,
) -> Option<(Vec<f32>, DimensionsZ)> {
    let mut min_z: Option<f32> = None;
    let mut max_z: Option<f32> = None;
    let mut heights_map: HashMap<CellKey, [[f32; 65]; 65]> = HashMap::default();

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                if landscape
                    .landscape_flags
                    .contains(LandscapeFlags::USES_VERTEX_HEIGHTS_AND_NORMALS)
                {
                    // get vertex data
                    // get data
                    let data = &landscape.vertex_heights.data;
                    let mut heights: [[f32; 65]; 65] = [[0.0; VERTEX_CNT]; VERTEX_CNT];
                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            heights[y][x] = data[y][x] as f32;
                        }
                    }

                    // decode
                    let mut offset: f32 = landscape.vertex_heights.offset;
                    for row in heights.iter_mut().take(VERTEX_CNT) {
                        for x in row.iter_mut().take(VERTEX_CNT) {
                            offset += *x;
                            *x = offset;
                        }
                        offset = row[0];
                    }

                    for row in &mut heights {
                        for height in row {
                            *height *= 8.0;

                            let z = *height;
                            if let Some(minz) = min_z {
                                if z < minz {
                                    min_z = Some(z);
                                }
                            } else {
                                min_z = Some(z);
                            }
                            if let Some(maxz) = max_z {
                                if z > maxz {
                                    max_z = Some(z);
                                }
                            } else {
                                max_z = Some(z);
                            }
                        }
                    }

                    heights_map.insert((cx, cy), heights);
                }
            }
        }
    }

    let min_z = min_z?;
    let max_z = max_z?;
    let dimensions_z = DimensionsZ { min_z, max_z };

    let heights = height_map_to_pixel_heights(dimensions, dimensions_z, heights_map);

    Some((heights, dimensions_z))
}

fn height_map_to_pixel_heights(
    dimensions: &Dimensions,
    dimensions_z: DimensionsZ,
    heights_map: HashMap<CellKey, [[f32; 65]; 65]>,
) -> Vec<f32> {
    // dimensions
    let max_x = dimensions.max_x;
    let min_x = dimensions.min_x;
    let max_y = dimensions.max_y;
    let min_y = dimensions.min_y;

    let size = dimensions.pixel_size(VERTEX_CNT);
    // hack to paint unset tiles
    let mut pixels = vec![dimensions_z.min_z - 1_f32; size];

    for cy in min_y..max_y + 1 {
        for cx in min_x..max_x + 1 {
            if let Some(heights) = heights_map.get(&(cx, cy)) {
                // look up heightmap
                for (y, row) in heights.iter().rev().enumerate() {
                    for (x, value) in row.iter().enumerate() {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let i = (ty * dimensions.stride(VERTEX_CNT)) + tx;
                        pixels[i] = *value;
                    }
                }
            } else {
                for y in 0..VERTEX_CNT {
                    for x in 0..VERTEX_CNT {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let i = (ty * dimensions.stride(VERTEX_CNT)) + tx;
                        pixels[i] = dimensions_z.min_z - 1_f32;
                    }
                }
            }
        }
    }

    pixels
}