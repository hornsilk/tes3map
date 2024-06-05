use std::env;

use egui::{Color32, pos2, Pos2, Rect, Sense};

use overlay::{grid::get_grid_shapes, regions::get_region_shapes};

use crate::*;
use crate::overlay::cities::get_cities_shapes;
use crate::overlay::travel::get_travel_shapes;

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // on start, we check the current folder for esps
        if self.data_files.is_none() {
            if let Ok(cwd) = env::current_dir() {
                // load once
                self.data_files = Some(cwd.clone());
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui
                        .hyperlink_to("Github repo", "https://github.com/rfuzzo/tes3map")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });

                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        // right panel
        egui::SidePanel::right("cell_panel").show(ctx, |ui| match self.side_panel_view {
            app::ESidePanelView::Plugins => self.plugins_panel(ui, ctx),
            app::ESidePanelView::Cells => self.cell_panel(ui, ctx),
        });

        // footer
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // Status Bar
            ui.horizontal(|ui| {
                // map bounds
                ui.label(format!(
                    "({},{}) - ({},{})",
                    self.dimensions.min_x,
                    self.dimensions.min_y,
                    self.dimensions.max_x,
                    self.dimensions.max_y
                ));
                ui.separator();
                ui.label(get_cell_name(&self.cell_records, self.hover_pos));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!(
                "Map (y: [{},{}]; x: [{},{}]; z: [{},{}])",
                self.dimensions.min_y,
                self.dimensions.max_y,
                self.dimensions.min_x,
                self.dimensions.max_x,
                self.dimensions_z.min_z,
                self.dimensions_z.max_z
            ));

            ui.separator();

            if self.heights.is_empty() {
                // settings
                egui::Frame::popup(ui.style())
                    .stroke(egui::Stroke::NONE)
                    .show(ui, |ui| {
                        ui.set_max_width(170.0);
                        egui::CollapsingHeader::new("Settings")
                            .show(ui, |ui| self.settings_ui(ui, ctx));
                    });

                return;
            }

            // painter
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

            // zoom
            if let Some(delta) = self.zoom_data.drag_delta.take() {
                self.zoom_data.drag_offset += delta.to_vec2();
            }

            if let Some(z) = self.zoom_data.zoom_delta.take() {
                let r = z - 1.0;
                let mut current_zoom = self.zoom_data.zoom;
                current_zoom += r;
                if current_zoom > 0.0 {
                    self.zoom_data.zoom = current_zoom;

                    // TODO offset the image for smooth zoom
                    if let Some(pointer_pos) = response.hover_pos() {
                        let d = pointer_pos * r;
                        self.zoom_data.drag_offset -= d.to_vec2();
                    }
                }
            }

            // TODO cut off pan at (0,0)
            // zoomed and panned canvas
            let min = self.zoom_data.drag_offset;
            let max =
                response.rect.max * self.zoom_data.zoom + self.zoom_data.drag_offset.to_vec2();
            let canvas = Rect::from_min_max(min, max);

            // transforms
            let pixel_width = self.dimensions.width() as f32 * self.dimensions.cell_size() as f32;
            let pixel_height = self.dimensions.height() as f32 * self.dimensions.cell_size() as f32;
            let to = canvas;
            let from: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(pixel_width, pixel_height));
            let to_screen = egui::emath::RectTransform::from_to(from, to);
            let from_screen = to_screen.inverse();

            // paint maps
            let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));
            // let rx = (response.rect.max.x - response.rect.min.x) / pixel_width;
            // let ry = (response.rect.max.y - response.rect.min.y) / pixel_height;
            // let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(rx, ry));

            // Background
            if let Some(texture) = &self.bg {
                painter.image(texture.into(), canvas, uv, Color32::WHITE);
            }

            // TODO Overlays
            if self.ui_data.overlay_paths {
                // let texture =
                //     get_color_pixels(&self.dimensions, &self.land_records, self.ui_data.alpha);
                // TODO texture handles
                // painter.image(texture, canvas, uv, Color32::WHITE);
            }
            if self.ui_data.overlay_region {
                let shapes = get_region_shapes(
                    to_screen,
                    &self.dimensions,
                    &self.regn_records,
                    &self.cell_records,
                );
                painter.extend(shapes);
            }
            if self.ui_data.overlay_grid {
                let shapes = get_grid_shapes(to_screen, &self.dimensions);
                painter.extend(shapes);
            }
            if self.ui_data.overlay_cities {
                let shapes = get_cities_shapes(to_screen, &self.dimensions, &self.cell_records);
                painter.extend(shapes);
            }
            if self.ui_data.overlay_travel {
                let shapes = get_travel_shapes(to_screen, &self.dimensions, &self.travel_edges);
                painter.extend(shapes);
            }

            // Responses

            // hover
            if let Some(pointer_pos) = response.hover_pos() {
                let cellsize = self.dimensions.cell_size();
                let transformed_position = from_screen * pointer_pos;

                // get pixel index
                let x = transformed_position.x as usize / cellsize;
                let y = transformed_position.y as usize / cellsize;
                // get cell grid
                let cx = self.dimensions.tranform_to_cell_x(x as i32);
                let cy = self.dimensions.tranform_to_cell_y(y as i32);

                self.hover_pos = (cx, cy);

                if let Some(value) = height_from_screen_space(
                    &self.heights,
                    &self.dimensions,
                    transformed_position.x as usize / VERTEX_CNT,
                    transformed_position.y as usize / VERTEX_CNT,
                ) {
                    self.info = format!("({cx}, {cy}), height: {value}",);
                }

                if self.ui_data.show_tooltips {
                    egui::show_tooltip(ui.ctx(), egui::Id::new("hover_tooltip"), |ui| {
                        ui.label(self.info.clone());
                    });
                }
            }

            // panning
            if response.drag_started() {
                if let Some(drag_start) = response.interact_pointer_pos() {
                    self.zoom_data.drag_start = drag_start;
                }
            } else if response.dragged() {
                if let Some(current_pos) = response.interact_pointer_pos() {
                    let delta = current_pos - self.zoom_data.drag_start.to_vec2();
                    self.zoom_data.drag_delta = Some(delta);
                    self.zoom_data.drag_start = current_pos;
                }
            }

            // zoom
            let delta = ctx.input(|i| i.zoom_delta());
            // let delta = response.input(|i| i.zoom_delta());
            if delta != 1.0 {
                self.zoom_data.zoom_delta = Some(delta);
            }
            if response.middle_clicked() {
                self.reset_zoom();
                self.reset_pan();
            }

            // Make sure we allocate what we used (everything)
            ui.expand_to_include_rect(painter.clip_rect());

            // settings
            // dumb ui hack
            let settings_rect = Rect::from_min_max(response.rect.min, pos2(0.0, 0.0));
            ui.put(settings_rect, egui::Label::new(""));

            egui::Frame::popup(ui.style())
                .stroke(egui::Stroke::NONE)
                .show(ui, |ui| {
                    ui.set_max_width(270.0);
                    egui::CollapsingHeader::new("Settings ")
                        .show(ui, |ui| self.settings_ui(ui, ctx));
                });

            response.context_menu(|ui| {
                if ui.button("Reset zoom").clicked() {
                    self.reset_pan();
                    self.reset_zoom();
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Save as image").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(_original_path) = file_option {
                        // logic here:

                        // TODO save image

                        // // if textures is selected, we just save that
                        // if self.ui_data.overlay_textures {
                        //     let max_texture_side = ctx.input(|i| i.max_texture_side);
                        //     let image = self.get_textured(max_texture_side);
                        //     if let Err(e) = save_image(&original_path, &image) {
                        //         println!("{}", e)
                        //     }
                        // } else {
                        //     // else we save the current overlayed image
                        //     if self.ui_data.overlay_terrain && self.ui_data.overlay_paths {
                        //         let background = self.get_background();
                        //         let foreground = self.get_foreground();

                        //         let layered_img = self.get_layered_image(background, foreground);
                        //         if let Err(e) = save_image(&original_path, &layered_img) {
                        //             println!("{}", e)
                        //         }
                        //     } else if self.ui_data.overlay_terrain {
                        //         // only save terrain
                        //         let background = self.get_background();
                        //         if let Err(e) = save_image(&original_path, &background) {
                        //             println!("{}", e)
                        //         }
                        //     } else if self.ui_data.overlay_paths {
                        //         // only save overlay
                        //         let foreground = self.get_foreground();
                        //         if let Err(e) = save_image(&original_path, &foreground) {
                        //             println!("{}", e)
                        //         }
                        //     }
                        // }
                    }

                    ui.close_menu();
                }

                if ui.button("Save active layers").clicked() {
                    let _file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    // if let Some(original_path) = file_option {
                    //     // save layers
                    //     if self.ui_data.overlay_textures {
                    //         // if textures is selected, save them to layer and main image
                    //         let max_texture_side = ctx.input(|i| i.max_texture_side);
                    //         let img = self.get_landscape_image(max_texture_side);
                    //         if let Err(e) = save_image(&original_path, &img) {
                    //             println!("{}", e)
                    //         }

                    //         if let Err(e) =
                    //             save_image(&append_to_filename(&original_path, "t"), &img)
                    //         {
                    //             println!("{}", e)
                    //         }

                    //         if self.ui_data.overlay_terrain {
                    //             // if only terrain is selected
                    //             let img = self.get_heightmap_image();
                    //             if let Err(e) =
                    //                 save_image(&append_to_filename(&original_path, "b"), &img)
                    //             {
                    //                 println!("{}", e)
                    //             }
                    //         }

                    //         if self.ui_data.overlay_paths {
                    //             // if only paths is selected
                    //             let img = self.get_overlay_path_image();
                    //             if let Err(e) =
                    //                 save_image(&append_to_filename(&original_path, "f"), &img)
                    //             {
                    //                 println!("{}", e)
                    //             }
                    //         }
                    //     } else if self.ui_data.overlay_terrain && self.ui_data.overlay_paths {
                    //         let img = self.get_heightmap_image();
                    //         if let Err(e) =
                    //             save_image(&append_to_filename(&original_path, "b"), &img)
                    //         {
                    //             println!("{}", e)
                    //         }

                    //         let img2 = self.get_overlay_path_image();
                    //         if let Err(e) =
                    //             save_image(&append_to_filename(&original_path, "f"), &img2)
                    //         {
                    //             println!("{}", e)
                    //         }

                    //         // save combined to main image
                    //         if self.ui_data.overlay_terrain && self.ui_data.overlay_paths {
                    //             let layered_img = self.get_layered_image(img, img2);
                    //             if let Err(e) = save_image(&original_path, &layered_img) {
                    //                 println!("{}", e)
                    //             }
                    //         }
                    //     } else if self.ui_data.overlay_terrain {
                    //         // if only terrain is selected
                    //         let img = self.get_heightmap_image();
                    //         if let Err(e) =
                    //             save_image(&append_to_filename(&original_path, "b"), &img)
                    //         {
                    //             println!("{}", e)
                    //         }

                    //         // save main
                    //         if let Err(e) = save_image(&original_path, &img) {
                    //             println!("{}", e)
                    //         }
                    //     } else if self.ui_data.overlay_paths {
                    //         // if only paths is selected
                    //         let img = self.get_overlay_path_image();
                    //         if let Err(e) =
                    //             save_image(&append_to_filename(&original_path, "f"), &img)
                    //         {
                    //             println!("{}", e)
                    //         }

                    //         // save f as main
                    //         if let Err(e) = save_image(&original_path, &img) {
                    //             println!("{}", e)
                    //         }
                    //     }
                    // }

                    ui.close_menu();
                }
            });
        });
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
