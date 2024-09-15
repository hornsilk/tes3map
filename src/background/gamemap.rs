use std::collections::HashMap;

use egui::{Color32, ColorImage};
use tes3::esp::Landscape;

use crate::{CellKey, Dimensions, GRID};

pub fn generate_map(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
) -> ColorImage {
    let height = dimensions.pixel_height(GRID);
    let width = dimensions.pixel_width(GRID);
    let size = height * width;

    // calculate map size
    let mut pixels: Vec<Color32> = Vec::with_capacity(size);

    for grid_y in 0..height {
        for grid_x in (0..width).rev() {
            // we can divide by grid to get the cell and subtract the bounds to get the cell coordinates
            let x = (grid_x / GRID) as i32 + dimensions.min_x;
            let y = (grid_y / GRID) as i32 + dimensions.min_y;

            // get LAND record
            let key = (x, y);
            if let Some(land) = landscape_records.get(&key) {
                // get remainder
                let hx = grid_x % GRID;
                let hy = grid_y % GRID;

                let heightmap = land.world_map_data.data.clone().to_vec();
                //pixels.push(get_map_color(heightmap[hy][hx] as f32));
                pixels.push(unpack_rgb(heightmap[hy][hx]));
            } else {
                pixels.push(Color32::TRANSPARENT);
            }
        }
    }

    pixels.reverse();

    ColorImage {
        pixels,
        size: [width, height],
    }
}

/// https://github.com/NullCascade/morrowind-mods/blob/master/User%20Interface%20Expansion/plugin_source/PatchWorldMap.cpp#L158
fn get_map_color(h: f32) -> Color32 {
    #[derive(Default)]
    struct MyColor {
        pub r: f32,
        pub g: f32,
        pub b: f32,
    }

    let height_data = 16.0 * h;
    let mut clipped_data = height_data / 2048.0;
    clipped_data = (-1.0_f32).max(clipped_data.min(1.0)); // rust wtf

    let mut pixel_color: MyColor = MyColor::default();
    // Above ocean level.
    if height_data >= 0.0 {
        // Darker heightmap threshold.
        if clipped_data > 0.3 {
            let base = (clipped_data - 0.3) * 1.428;
            pixel_color.r = 34.0 - base * 29.0;
            pixel_color.g = 25.0 - base * 20.0;
            pixel_color.b = 17.0 - base * 12.0;
        }
        // Lighter heightmap threshold.
        else {
            let mut base = clipped_data * 8.0;
            if clipped_data > 0.1 {
                base = clipped_data - 0.1 + 0.8;
            }
            pixel_color.r = 66.0 - base * 32.0;
            pixel_color.g = 48.0 - base * 23.0;
            pixel_color.b = 33.0 - base * 16.0;
        }
    }
    // Underwater, fade out towards the water color.
    else {
        pixel_color.r = 38.0 + clipped_data * 14.0;
        pixel_color.g = 56.0 + clipped_data * 20.0;
        pixel_color.b = 51.0 + clipped_data * 18.0;
    }

    Color32::from_rgb(
        pixel_color.r as u8,
        pixel_color.g as u8,
        pixel_color.b as u8,
    )
}

pub fn pack_rgb(c: Color32) -> i8 {
    // Reduce the range of r, g, b to 2 bits each (values between 0–3)
    let r = (c.r() >> 6) & 0b11; // 2 most significant bits of r
    let g = (c.g() >> 6) & 0b11; // 2 most significant bits of g
    let b = (c.b() >> 6) & 0b11; // 2 most significant bits of b

    // Combine the bits into a single i8 value
    let packed: u8 = (r << 4) | (g << 2) | b;

    // Cast the packed u8 to i8 (unsigned to signed)
    packed as i8
}

pub fn unpack_rgb(packed: i8) -> Color32 {
    // Extract the 2 bits for each channel
    let r = ((packed >> 4) & 0b11) << 6;
    let g = ((packed >> 2) & 0b11) << 6;
    let b = (packed & 0b11) << 6;

    // Return the unpacked values
    Color32::from_rgb(r as u8, g as u8, b as u8)
}
