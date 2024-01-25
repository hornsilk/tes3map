use crate::*;

use std::env;

use egui::{pos2, Color32, Pos2, Rect, Sense};

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.cwd.is_none() {
            if let Ok(cwd) = env::current_dir() {
                // load once
                self.cwd = Some(cwd.clone());
                self.load_folder(&cwd, ctx);
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load folder").clicked() {
                        self.open_folder(ctx);
                        ui.close_menu();
                    }

                    if ui.button("Load plugin").clicked() {
                        self.open_plugin(ctx);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        // egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
        //     ui.heading("Cells");
        //     ui.separator();
        // });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!(
                "Map (y: [{},{}]; x: [{},{}]; z: [{},{}])",
                self.dimensions.min_y,
                self.dimensions.max_y,
                self.dimensions.min_x,
                self.dimensions.max_x,
                self.dimensions.min_z,
                self.dimensions.max_z
            ));

            ui.separator();

            if self.heights.is_empty() {
                // Default UI
                ui.horizontal(|ui| {
                    if ui.button("Load plugin").clicked() {
                        self.open_plugin(ctx);
                    }

                    if ui.button("Load folder").clicked() {
                        self.open_folder(ctx);
                    }
                });

                // settings
                egui::Frame::popup(ui.style())
                    .stroke(egui::Stroke::NONE)
                    .show(ui, |ui| {
                        ui.set_max_width(170.0);
                        egui::CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
                    });

                return;
            }

            // painter
            // let clip_rect = ui.available_rect_before_wrap();
            // let painter = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip_rect);
            // let response = painter.ctx();

            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

            // panning and zooming
            if let Some(delta) = self.zoom_data.drag_delta.take() {
                self.zoom_data.drag_offset += delta.to_vec2();
            }

            // move to center zoom
            if let Some(z) = self.zoom_data.zoom_delta.take() {
                let r = z - 1.0;
                self.zoom_data.zoom += r;

                // TODO offset the image for smooth zoom
                if let Some(pointer_pos) = response.hover_pos() {
                    let d = pointer_pos * r;
                    self.zoom_data.drag_offset -= d.to_vec2();
                }
            }

            // TODO cut off pan at (0,0)
            let min = self.zoom_data.drag_offset;
            let max =
                response.rect.max * self.zoom_data.zoom + self.zoom_data.drag_offset.to_vec2();
            let canvas = Rect::from_min_max(min, max);
            let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));

            // transforms
            let to = canvas;
            let from = egui::Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(
                    self.dimensions.width() as f32,
                    self.dimensions.height() as f32,
                ),
            );
            let to_screen = egui::emath::RectTransform::from_to(from, to);
            let from_screen = to_screen.inverse();

            // paint maps
            if self.ui_data.overlay_terrain {
                if let Some(texture) = &self.background {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }
            if self.ui_data.overlay_textures {
                if let Some(texture) = &self.textured {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }
            if self.ui_data.overlay_paths {
                if let Some(texture) = &self.foreground {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }

            // Responses

            // hover
            if let Some(pointer_pos) = response.hover_pos() {
                let canvas_pos = from_screen * pointer_pos;

                let canvas_pos_x = canvas_pos.x as usize;
                let canvas_pos_y = canvas_pos.y as usize;
                let i = ((canvas_pos_y * self.dimensions.width()) + canvas_pos_x) as usize;

                if i < self.heights.len() {
                    let value = self.heights[i];

                    let x = canvas_pos.x as usize / VERTEX_CNT;
                    let y = canvas_pos.y as usize / VERTEX_CNT;
                    let cx = self.dimensions.tranform_to_cell_x(x as i32);
                    let cy = self.dimensions.tranform_to_cell_y(y as i32);
                    self.info = format!("({}, {}), height: {}", cx, cy, value);
                }

                if self.ui_data.show_tooltips {
                    egui::show_tooltip(ui.ctx(), egui::Id::new("my_tooltip"), |ui| {
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
            // TODO dumb hack
            let settings_rect = egui::Rect::from_min_max(response.rect.min, pos2(0.0, 0.0));
            ui.put(settings_rect, egui::Label::new(""));

            egui::Frame::popup(ui.style())
                .stroke(egui::Stroke::NONE)
                .show(ui, |ui| {
                    ui.set_max_width(270.0);
                    egui::CollapsingHeader::new("Settings ").show(ui, |ui| self.options_ui(ui));
                });

            response.context_menu(|ui| {
                if ui.button("Save as image").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(original_path) = file_option {
                        // combined
                        let img = self.get_background();
                        let img2 = self.get_foreground();
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }
                    }

                    ui.close_menu();
                }

                if ui.button("Save as layers").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(original_path) = file_option {
                        // save layers
                        let img = self.get_background();
                        let mut new_path = append_number_to_filename(&original_path, 1);
                        match save_image(new_path, &img) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        let img2 = self.get_foreground();
                        new_path = append_number_to_filename(&original_path, 2);
                        match save_image(new_path, &img2) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        // combined
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }
                    }

                    ui.close_menu();
                }
            });
        });
    }
}
