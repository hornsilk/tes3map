use std::{collections::HashMap, path::PathBuf};

use egui::{pos2, Color32, ColorImage, Pos2, Rounding, Shape, Stroke};
use image::{imageops, ImageError};
use log::{debug, error};
use tes3::esp::{Landscape, Region};

use background::{
    gamemap::generate_map, heightmap::generate_heightmap, landscape::compute_landscape_image, ptmap::generate_ptmap,
};
use overlay::{paths::get_overlay_path_image, regions::get_region_shapes};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ESidePanelView {
    #[default]
    Plugins,
    Cells,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TooltipInfo {
    pub key: CellKey,
    pub height: f32,
    pub region: String,
    pub cell_name: String,
    pub conflicts: Vec<u64>,
    pub debug: String,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TemplateApp {
    pub data_files: Option<PathBuf>,
    pub ui_data: SavedData,

    // ui
    #[serde(skip)]
    pub zoom_data: ZoomData,
    #[serde(skip)]
    pub dimensions: Dimensions,

    // tes3
    #[serde(skip)]
    pub plugins: Option<Vec<PluginViewModel>>,

    // runtime data
    #[serde(skip)]
    pub land_records: HashMap<CellKey, Landscape>,
    #[serde(skip)]
    pub ltex_records: HashMap<u32, LandscapeTexture>,
    #[serde(skip)]
    pub regn_records: HashMap<String, Region>,
    #[serde(skip)]
    pub cell_records: HashMap<CellKey, Cell>,
   
    // intervention spells
    #[serde(skip)]
    pub almsivi_interventions: HashMap<CellKey, Cell>,
    #[serde(skip)]
    pub divine_interventions: HashMap<CellKey, Cell>,
    #[serde(skip)]
    pub kyne_interventions: HashMap<CellKey, Cell>,

    pub intervention_engine: String,
    
    // overlays
    #[serde(skip)]
    pub travel_edges: HashMap<String, Vec<(CellKey, CellKey)>>,
    #[serde(skip)]
    pub cell_conflicts: HashMap<CellKey, Vec<u64>>,
    // textures in memory
    #[serde(skip)]
    pub background_handle: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub paths_handle: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub heights: Vec<f32>,
    #[serde(skip)]
    pub texture_map_resolution: usize,
    #[serde(skip)]
    pub texture_map: HashMap<String, ImageBuffer>,

    // runtime data
    #[serde(skip)]
    pub side_panel_view: ESidePanelView,
    #[serde(skip)]
    pub runtime_data: RuntimeData,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    pub fn reload_paths(&mut self, ctx: &egui::Context) {
        let image = get_overlay_path_image(&self.dimensions, &self.land_records);
        self.paths_handle = Some(ctx.load_texture("paths", image, Default::default()));
    }

    pub fn populate_texture_map(&mut self, max_texture_side: usize) {
        // if the resolution is the same, no need to reload
        let max_texture_resolution = self.dimensions.get_max_texture_resolution(max_texture_side);
        if max_texture_resolution > self.texture_map_resolution
            && self.texture_map_resolution == self.ui_data.landscape_settings.texture_size
        {
            debug!("Texture resolution is the same, no need to reload");
            return;
        }

        // otherwise check if possible
        let cell_size = self.ui_data.landscape_settings.cell_size();
        let width = self.dimensions.pixel_width(cell_size);
        let height = self.dimensions.pixel_height(cell_size);
        if width > max_texture_side || height > max_texture_side {
            error!(
                "Texture size too large: (width: {}, height: {}), supported side: {}, max_texture_side: {}",
                width, height, max_texture_side, max_texture_resolution
            );

            debug!(
                "texture_size {}",
                self.ui_data.landscape_settings.texture_size
            );
            debug!("Resetting texture size to 16");

            self.ui_data.landscape_settings.texture_size = 16.min(max_texture_resolution);

            // rfd messagebox
            let msg = format!(
                "Texture size too large, supported side: {}",
                max_texture_resolution
            );
            rfd::MessageDialog::new()
                .set_title("Error")
                .set_description(msg)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }

        self.texture_map.clear();
        let texture_size = self.ui_data.landscape_settings.texture_size;
        self.texture_map_resolution = texture_size;

        debug!("Populating texture map with resolution: {}", texture_size);

        for cy in self.dimensions.min_y..self.dimensions.max_y + 1 {
            for cx in self.dimensions.min_x..self.dimensions.max_x + 1 {
                if let Some(landscape) = self.land_records.get(&(cx, cy)) {
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

                                    // load texture
                                    if let Some(ltex) = self.ltex_records.get(&key) {
                                        // texture name
                                        let texture_name = ltex.file_name.clone();
                                        if self.texture_map.contains_key(&texture_name) {
                                            continue;
                                        }

                                        if let Ok(tex) = load_texture(&self.data_files, ltex) {
                                            // resize the image
                                            let image = imageops::resize(
                                                &tex,
                                                texture_size as u32,
                                                texture_size as u32,
                                                imageops::FilterType::CatmullRom,
                                            );

                                            // let size = [image.width() as _, image.height() as _];
                                            // let image_buffer = image.to_rgba8();
                                            // let pixels = image_buffer.as_flat_samples();
                                            // return Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));

                                            info!("Loaded texture: {}", ltex.file_name);
                                            self.texture_map.insert(texture_name, image);
                                        } else {
                                            error!("Failed to load texture: {}", ltex.file_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Assigns landscape_records, dimensions and pixels
    pub fn reload_background(
        &mut self,
        ctx: &egui::Context,
        new_dimensions: Option<Dimensions>,
        recalculate_dimensions: bool,
        recalculate_heights: bool,
    ) {
        // calculate dimensions
        if let Some(dimensions) = new_dimensions {
            self.dimensions = dimensions.clone();
        } else if recalculate_dimensions {
            if let Some(dims) = calculate_dimensions(&self.dimensions, &self.land_records) {
                self.dimensions = dims;
            } else {
                return;
            }
        }

        // calculate heights
        if recalculate_heights {
            if let Some(heights) = calculate_heights(&self.land_records, &mut self.dimensions) {
                self.heights = heights;
            }
        }

        let image = match self.ui_data.background {
            EBackground::None => None,
            EBackground::Landscape => {
                let max_texture_side = ctx.input(|i| i.max_texture_side);
                self.populate_texture_map(max_texture_side);

                Some(self.get_landscape_image())
            }
            EBackground::HeightMap => Some(self.get_heightmap_image()),
            EBackground::GameMap => Some(self.get_gamemap_image()),
            EBackground::PTMap => Some(self.get_ptmap_image()),
        };

        if let Some(image) = image {
            self.background_handle =
                Some(ctx.load_texture("background", image, Default::default()));
        } else {
            self.background_handle = None;
        }
    }

    // Shortcuts

    pub fn get_heightmap_image(&mut self) -> ColorImage {
        generate_heightmap(
            &self.heights,
            &self.dimensions,
            &self.ui_data.heightmap_settings,
        )
    }

    pub fn get_gamemap_image(&mut self) -> ColorImage {
        generate_map(&self.dimensions, &self.land_records)
    }

    pub fn get_ptmap_image(&mut self) -> ColorImage {
        generate_ptmap(&self.dimensions, &self.plugins)
    }

    pub fn get_landscape_image(&mut self) -> ColorImage {
        compute_landscape_image(
            &self.ui_data.landscape_settings,
            &self.dimensions,
            &self.land_records,
            &self.ltex_records,
            &self.heights,
            &self.texture_map,
        )
    }

    // UI methods
    pub fn reset_zoom(&mut self) {
        self.zoom_data.zoom = 1.0;
    }

    pub fn reset_pan(&mut self) {
        self.zoom_data.drag_delta = None;
        self.zoom_data.drag_offset = Pos2::default();
        self.zoom_data.drag_start = Pos2::default();
    }

    pub fn save_image(&mut self, ctx: &egui::Context) -> Result<(), ImageError> {
        // construct default name from the first plugin name then the background type abbreviated
        let background_name = match self.ui_data.background {
            EBackground::None => "",
            EBackground::Landscape => "l",
            EBackground::HeightMap => "h",
            EBackground::GameMap => "g",
            EBackground::PTMap => "p",
        };
        let first_plugin = self
            .plugins
            .as_ref()
            .unwrap()
            .iter()
            .filter(|p| p.enabled)
            .nth(0)
            .unwrap();
        let plugin_name = first_plugin.get_name();
        let defaultname = format!("{}_{}.png", plugin_name, background_name);

        let file_option = rfd::FileDialog::new()
            .add_filter("png", &["png"])
            .set_file_name(defaultname)
            .save_file();

        if let Some(original_path) = file_option {
            let mut background = None;
            match self.ui_data.background {
                EBackground::None => {}
                EBackground::Landscape => {
                    let max_texture_side = ctx.input(|i| i.max_texture_side);
                    self.populate_texture_map(max_texture_side);
                    background = Some(self.get_landscape_image());
                }
                EBackground::HeightMap => {
                    background = Some(self.get_heightmap_image());
                }
                EBackground::GameMap => {
                    background = Some(self.get_gamemap_image());
                }
                EBackground::PTMap => {
                    background = Some(self.get_ptmap_image());
                }
            }

            let any_overlay = self.ui_data.overlay_region
                || self.ui_data.overlay_grid
                || self.ui_data.overlay_cities
                || self.ui_data.overlay_alm_interventions
                || self.ui_data.overlay_div_interventions
                || self.ui_data.overlay_kyn_interventions
                || self.ui_data.overlay_travel
                || self.ui_data.overlay_conflicts
                || !self.runtime_data.selected_ids.is_empty()
                || self.runtime_data.pivot_id.is_some();

            let mut target_width = background.as_ref().map(|b| b.size[0]).unwrap_or(0);
            let mut target_height = background.as_ref().map(|b| b.size[1]).unwrap_or(0);

            if self.ui_data.overlay_paths {
                let paths_size = self.dimensions.pixel_size_tuple(VERTEX_CNT);
                target_width = target_width.max(paths_size[0]);
                target_height = target_height.max(paths_size[1]);
            }

            if any_overlay {
                let min_size = self.dimensions.pixel_size_tuple(VERTEX_CNT);
                target_width = target_width.max(min_size[0]);
                target_height = target_height.max(min_size[1]);
            } else if target_width == 0 {
                let default_size = self.dimensions.pixel_size_tuple(VERTEX_CNT);
                target_width = default_size[0];
                target_height = default_size[1];
            }

            let mut pixmap = tiny_skia::Pixmap::new(target_width as u32, target_height as u32)
                .ok_or_else(|| {
                    ImageError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to allocate canvas pixmap",
                    ))
                })?;

            // 1. Background
            if let Some(bg) = &background {
                draw_color_image_to_canvas(&mut pixmap, bg);
            }

            // 2. Paths
            if self.ui_data.overlay_paths {
                let fg = get_overlay_path_image(&self.dimensions, &self.land_records);
                draw_color_image_to_canvas(&mut pixmap, &fg);
            }

            // 3. Overlays
            if any_overlay {
                let real_width = self.dimensions.width() as f32;
                let real_height = self.dimensions.height() as f32;
                let from: Rect =
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(real_width, real_height));

                let canvas_width = target_width as f32;
                let canvas_height = target_height as f32;
                let transform = RectTransform::from_to(
                    from,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(canvas_width, canvas_height)),
                );

                let mut all_shapes = vec![];

                // order is: paths, regions, grid, interventions, cities, travel, conflicts, selections
                if self.ui_data.overlay_region {
                    let shapes = get_region_shapes(
                        transform,
                        &self.dimensions,
                        &self.regn_records,
                        &self.cell_records,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_grid {
                    let shapes = overlay::grid::get_grid_shapes(transform, &self.dimensions);
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_alm_interventions {
                    let shapes = overlay::interventions::get_intervention_shapes(
                        transform,
                        &self.dimensions,
                        &self.almsivi_interventions,
                        &self.cell_records,
                        "almsivi",
                        &self.intervention_engine,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_div_interventions {
                    let shapes = overlay::interventions::get_intervention_shapes(
                        transform,
                        &self.dimensions,
                        &self.divine_interventions,
                        &self.cell_records,
                        "divine",
                        &self.intervention_engine,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_kyn_interventions {
                    let shapes = overlay::interventions::get_intervention_shapes(
                        transform,
                        &self.dimensions,
                        &self.kyne_interventions,
                        &self.cell_records,
                        "kyne",
                        &self.intervention_engine,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_cities {
                    let shapes = overlay::cities::get_cities_shapes(
                        transform,
                        &self.dimensions,
                        &self.cell_records,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_travel {
                    let shapes = overlay::travel::get_travel_shapes(
                        transform,
                        &self.dimensions,
                        &self.travel_edges,
                    );
                    all_shapes.extend(shapes);
                }
                if self.ui_data.overlay_conflicts {
                    let shapes = overlay::conflicts::get_conflict_shapes(
                        transform,
                        &self.dimensions,
                        &self.cell_conflicts,
                    );
                    all_shapes.extend(shapes);
                }
                for key in &self.runtime_data.selected_ids {
                    let rect = get_rect_at_cell(&self.dimensions, transform, *key);
                    let shape =
                        Shape::rect_stroke(rect, Rounding::default(), Stroke::new(4.0, Color32::RED));
                    all_shapes.push(shape);
                }
                if let Some(pivot_id) = self.runtime_data.pivot_id {
                    let rect = get_rect_at_cell(&self.dimensions, transform, pivot_id);
                    let shape = Shape::rect_stroke(
                        rect,
                        Rounding::default(),
                        Stroke::new(4.0, Color32::LIGHT_BLUE),
                    );
                    all_shapes.push(shape);
                }

                // draw the shapes
                for shape in all_shapes {
                    render_shape(&shape, &mut pixmap);
                }
            }

            let png_data = pixmap.encode_png().map_err(|e| {
                ImageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to encode PNG: {}", e),
                ))
            })?;
            std::fs::write(&original_path, png_data)?;

            rfd::MessageDialog::new()
                .set_title("Info")
                .set_description("Image saved successfully")
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }

        Ok(())
    }
}

fn color_image_to_pixmap(color_image: &ColorImage) -> Option<tiny_skia::Pixmap> {
    let width = color_image.width() as u32;
    let height = color_image.height() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let data = pixmap.data_mut();
    for (i, pixel) in color_image.pixels.iter().enumerate() {
        let [r, g, b, a] = pixel.to_srgba_unmultiplied();
        if a > 0 {
            let skia_color = tiny_skia::Color::from_rgba8(r, g, b, a);
            let premul = skia_color.to_color_u8();
            data[i * 4] = premul.red();
            data[i * 4 + 1] = premul.green();
            data[i * 4 + 2] = premul.blue();
            data[i * 4 + 3] = premul.alpha();
        } else {
            data[i * 4] = 0;
            data[i * 4 + 1] = 0;
            data[i * 4 + 2] = 0;
            data[i * 4 + 3] = 0;
        }
    }
    Some(pixmap)
}

fn draw_color_image_to_canvas(canvas: &mut tiny_skia::Pixmap, color_image: &ColorImage) {
    if let Some(src_pixmap) = color_image_to_pixmap(color_image) {
        if canvas.width() == src_pixmap.width() && canvas.height() == src_pixmap.height() {
            canvas.draw_pixmap(
                0,
                0,
                src_pixmap.as_ref(),
                &tiny_skia::PixmapPaint {
                    opacity: 1.0,
                    blend_mode: tiny_skia::BlendMode::SourceOver,
                    quality: tiny_skia::FilterQuality::Nearest,
                },
                tiny_skia::Transform::identity(),
                None,
            );
        } else {
            let scale_x = canvas.width() as f32 / src_pixmap.width() as f32;
            let scale_y = canvas.height() as f32 / src_pixmap.height() as f32;
            canvas.draw_pixmap(
                0,
                0,
                src_pixmap.as_ref(),
                &tiny_skia::PixmapPaint {
                    opacity: 1.0,
                    blend_mode: tiny_skia::BlendMode::SourceOver,
                    quality: tiny_skia::FilterQuality::Bilinear,
                },
                tiny_skia::Transform::from_scale(scale_x, scale_y),
                None,
            );
        }
    }
}

fn render_shape(shape: &Shape, pixmap: &mut tiny_skia::Pixmap) {
    match shape {
        Shape::Noop => {}
        Shape::Vec(shapes) => {
            for s in shapes {
                render_shape(s, pixmap);
            }
        }
        Shape::Rect(rect_shape) => {
            if let Some(skia_rect) = tiny_skia::Rect::from_xywh(
                rect_shape.rect.min.x,
                rect_shape.rect.min.y,
                rect_shape.rect.width(),
                rect_shape.rect.height(),
            ) {
                // Filled
                if rect_shape.fill != Color32::TRANSPARENT {
                    let [r, g, b, a] = rect_shape.fill.to_srgba_unmultiplied();
                    if a > 0 {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color_rgba8(r, g, b, a);
                        paint.anti_alias = true;
                        pixmap.fill_rect(skia_rect, &paint, tiny_skia::Transform::identity(), None);
                    }
                }
                // Stroked
                if rect_shape.stroke.width > 0.0 && rect_shape.stroke.color != Color32::TRANSPARENT {
                    let [r, g, b, a] = rect_shape.stroke.color.to_srgba_unmultiplied();
                    if a > 0 {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color_rgba8(r, g, b, a);
                        paint.anti_alias = true;
                        let mut stroke = tiny_skia::Stroke::default();
                        stroke.width = rect_shape.stroke.width;
                        stroke.line_join = tiny_skia::LineJoin::Miter;
                        let mut pb = tiny_skia::PathBuilder::new();
                        pb.move_to(skia_rect.left(), skia_rect.top());
                        pb.line_to(skia_rect.right(), skia_rect.top());
                        pb.line_to(skia_rect.right(), skia_rect.bottom());
                        pb.line_to(skia_rect.left(), skia_rect.bottom());
                        pb.close();
                        if let Some(path) = pb.finish() {
                            pixmap.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
        Shape::LineSegment { points, stroke } => {
            if stroke.width > 0.0 {
                let color = match stroke.color {
                    egui::epaint::ColorMode::Solid(c) => c,
                    egui::epaint::ColorMode::UV(_) => Color32::TRANSPARENT,
                };
                if color != Color32::TRANSPARENT {
                    let [r, g, b, a] = color.to_srgba_unmultiplied();
                    if a > 0 {
                        let mut pb = tiny_skia::PathBuilder::new();
                        pb.move_to(points[0].x, points[0].y);
                        pb.line_to(points[1].x, points[1].y);
                        if let Some(path) = pb.finish() {
                            let mut paint = tiny_skia::Paint::default();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            let mut skia_stroke = tiny_skia::Stroke::default();
                            skia_stroke.width = stroke.width;
                            skia_stroke.line_cap = tiny_skia::LineCap::Round;
                            pixmap.stroke_path(&path, &paint, &skia_stroke, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
        Shape::Path(path_shape) => {
            if path_shape.points.len() >= 2 {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(path_shape.points[0].x, path_shape.points[0].y);
                for pt in &path_shape.points[1..] {
                    pb.line_to(pt.x, pt.y);
                }
                if path_shape.closed {
                    pb.close();
                }
                if let Some(path) = pb.finish() {
                    // Fill
                    if path_shape.fill != Color32::TRANSPARENT {
                        let [r, g, b, a] = path_shape.fill.to_srgba_unmultiplied();
                        if a > 0 {
                            let mut paint = tiny_skia::Paint::default();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            pixmap.fill_path(&path, &paint, tiny_skia::FillRule::EvenOdd, tiny_skia::Transform::identity(), None);
                        }
                    }
                    // Stroke
                    if path_shape.stroke.width > 0.0 {
                        let color = match path_shape.stroke.color {
                            egui::epaint::ColorMode::Solid(c) => c,
                            egui::epaint::ColorMode::UV(_) => Color32::TRANSPARENT,
                        };
                        if color != Color32::TRANSPARENT {
                            let [r, g, b, a] = color.to_srgba_unmultiplied();
                            if a > 0 {
                                let mut paint = tiny_skia::Paint::default();
                                paint.set_color_rgba8(r, g, b, a);
                                paint.anti_alias = true;
                                let mut skia_stroke = tiny_skia::Stroke::default();
                                skia_stroke.width = path_shape.stroke.width;
                                skia_stroke.line_join = tiny_skia::LineJoin::Round;
                                pixmap.stroke_path(&path, &paint, &skia_stroke, tiny_skia::Transform::identity(), None);
                            }
                        }
                    }
                }
            }
        }
        Shape::Circle(circle_shape) => {
            if circle_shape.radius > 0.0 {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.push_circle(circle_shape.center.x, circle_shape.center.y, circle_shape.radius);
                if let Some(path) = pb.finish() {
                    if circle_shape.fill != Color32::TRANSPARENT {
                        let [r, g, b, a] = circle_shape.fill.to_srgba_unmultiplied();
                        if a > 0 {
                            let mut paint = tiny_skia::Paint::default();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
                        }
                    }
                    if circle_shape.stroke.width > 0.0 && circle_shape.stroke.color != Color32::TRANSPARENT {
                        let [r, g, b, a] = circle_shape.stroke.color.to_srgba_unmultiplied();
                        if a > 0 {
                            let mut paint = tiny_skia::Paint::default();
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            let mut skia_stroke = tiny_skia::Stroke::default();
                            skia_stroke.width = circle_shape.stroke.width;
                            pixmap.stroke_path(&path, &paint, &skia_stroke, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::epaint::PathStroke;

    #[test]
    fn test_color_image_to_pixmap_and_draw() {
        let mut color_img = ColorImage::new([10, 10], Color32::from_rgb(100, 150, 200));
        color_img.pixels[0] = Color32::from_rgba_unmultiplied(255, 0, 0, 128);

        let mut canvas = tiny_skia::Pixmap::new(20, 20).unwrap();
        draw_color_image_to_canvas(&mut canvas, &color_img);

        // Canvas should have non-zero pixels
        assert!(canvas.data().iter().any(|&b| b != 0));

        // Test PNG encoding
        let png_bytes = canvas.encode_png().expect("PNG encoding failed");
        assert!(png_bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
    }

    #[test]
    fn test_render_all_shape_types() {
        let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();

        // 1. Rect filled & stroked
        let rect = egui::Rect::from_min_max(pos2(10.0, 10.0), pos2(40.0, 40.0));
        let rect_shape = Shape::rect_filled(rect, Rounding::default(), Color32::from_rgba_unmultiplied(0, 255, 0, 10));
        render_shape(&rect_shape, &mut pixmap);
        let stroke_shape = Shape::rect_stroke(rect, Rounding::default(), Stroke::new(2.0, Color32::BLUE));
        render_shape(&stroke_shape, &mut pixmap);

        // 2. Diagonal Line segment (like travel routes)
        let line_shape = Shape::LineSegment {
            points: [pos2(5.0, 5.0), pos2(95.0, 85.0)],
            stroke: PathStroke::new(3.0, Color32::GOLD),
        };
        render_shape(&line_shape, &mut pixmap);

        // 3. Convex polygon (like Voronoi and Almsivi / Divine / Kyne icons)
        let tri = vec![pos2(50.0, 10.0), pos2(70.0, 40.0), pos2(30.0, 40.0)];
        let poly_shape = Shape::convex_polygon(tri, Color32::from_rgb(180, 25, 25), Stroke::new(1.5, Color32::BLACK));
        render_shape(&poly_shape, &mut pixmap);

        // 4. Circle (intervention node dot)
        let circle_shape = Shape::circle_filled(pos2(50.0, 50.0), 4.0, Color32::from_rgb(0, 100, 0));
        render_shape(&circle_shape, &mut pixmap);

        // Verify that shapes were drawn
        assert!(pixmap.data().iter().any(|&b| b != 0));

        let png = pixmap.encode_png().expect("PNG encoding failed");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
