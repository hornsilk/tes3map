use std::{collections::HashMap, path::PathBuf};

use egui::{Color32, ColorImage};
use log::info;
use tes3::esp::{Landscape, LandscapeFlags, LandscapeTexture};

use crate::{
    CellKey, DEFAULT_COLOR, Dimensions, GRID_SIZE, height_from_screen_space,
    load_texture, overlay_colors_with_alpha, TEXTURE_MAX_SIZE, VERTEX_CNT,
};

/// Compute a landscape image from the given landscape records and texture map.
pub fn compute_landscape_image(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
    ltex_records: &HashMap<u32, LandscapeTexture>,
    data_files: &Option<PathBuf>,
    heights: &[f32],
) -> Option<ColorImage> {
    let d = dimensions;
    let size = d.pixel_size(d.cell_size());
    let size_tuple = d.pixel_size_tuple(d.cell_size());
    let width = size_tuple[0];
    let height = size_tuple[1];
    info!(
        "Generating textured image with size {} (width: {}, height: {})",
        size, width, height,
    );

    let mut pixels_color = vec![Color32::TRANSPARENT; size];

    let mut texture_map = HashMap::new();

    for cy in d.min_y..d.max_y + 1 {
        for cx in d.min_x..d.max_x + 1 {
            if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                if landscape
                    .landscape_flags
                    .contains(LandscapeFlags::USES_TEXTURES)
                {
                    {
                        let data = &landscape.texture_indices.data;
                        for gx in 0..GRID_SIZE {
                            for gy in 0..GRID_SIZE {
                                let dx = (4 * (gy % 4)) + (gx % 4);
                                let dy = (4 * (gy / 4)) + (gx / 4);

                                let key = data[dy][dx] as u32;

                                // lazy load texture
                                let texture = texture_map.entry(key).or_insert_with(|| {
                                    // load texture
                                    if let Some(ltex) = ltex_records.get(&key) {
                                        if let Some(tex) = load_texture(data_files, ltex) {
                                            return Some(tex);
                                        }
                                    }

                                    None
                                });
                                if texture.is_none() {
                                    continue;
                                }

                                // textures per tile
                                for x in 0..d.texture_size {
                                    for y in 0..d.texture_size {
                                        let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                            + gx * d.texture_size
                                            + x;
                                        let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                            + (GRID_SIZE - 1 - gy) * d.texture_size
                                            + y;

                                        let i = (ty * d.stride(d.cell_size())) + tx;

                                        // pick every nth pixel from the texture to downsize
                                        let sx = x * (TEXTURE_MAX_SIZE / d.texture_size);
                                        let sy = y * (TEXTURE_MAX_SIZE / d.texture_size);
                                        let index = (sy * d.texture_size) + sx;

                                        let mut color = texture.as_ref().unwrap().pixels[index];

                                        // blend color when under water
                                        let screenx = tx * VERTEX_CNT / d.cell_size();
                                        let screeny = ty * VERTEX_CNT / d.cell_size();

                                        if let Some(height) = height_from_screen_space(
                                            heights, dimensions, screenx, screeny,
                                        ) {
                                            if height < 0_f32 {
                                                let a = 0.5;

                                                color = overlay_colors_with_alpha(
                                                    color,
                                                    Color32::BLUE,
                                                    a,
                                                );
                                            }
                                        }

                                        pixels_color[i] = color;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // no landscape
                for gx in 0..GRID_SIZE {
                    for gy in 0..GRID_SIZE {
                        // textures per tile
                        for x in 0..d.texture_size {
                            for y in 0..d.texture_size {
                                let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                    + gx * d.texture_size
                                    + x;
                                let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                    + gy * d.texture_size
                                    + y;

                                let i = (ty * d.stride(d.cell_size())) + tx;

                                pixels_color[i] = DEFAULT_COLOR;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut img = ColorImage::new(d.pixel_size_tuple(d.cell_size()), Color32::GOLD);
    img.pixels = pixels_color;
    Some(img)
}