use std::path::{Path, PathBuf};

use eframe::egui;

struct MatataView {
    texture: Option<egui::TextureHandle>,
    image_size: Option<egui::Vec2>,
    path: Option<PathBuf>,
    error: Option<String>,
    zoom: f32,
}

impl MatataView {
    fn new(ctx: &egui::Context, path: Option<PathBuf>) -> Self {
        let mut viewer = Self {
            texture: None,
            image_size: None,
            path: None,
            error: None,
            zoom: 1.0,
        };
        if let Some(path) = path {
            viewer.open_image(ctx, path);
        }
        viewer
    }

    fn open_image(&mut self, ctx: &egui::Context, path: PathBuf) {
        let result = load_image(&path).map(|image| {
            let size = egui::vec2(image.size[0] as f32, image.size[1] as f32);
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("image")
                .to_owned();
            (
                ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
                size,
            )
        });

        match result {
            Ok((texture, size)) => {
                self.texture = Some(texture);
                self.image_size = Some(size);
                self.path = Some(path);
                self.error = None;
                self.zoom = 1.0;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn choose_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["jpg", "jpeg", "png"])
            .pick_file()
        {
            self.open_image(ctx, path);
        }
    }
}

impl eframe::App for MatataView {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if let Some(path) = dropped_files.first().map(|file| file.path().to_path_buf()) {
            self.open_image(&ctx, path);
        }
        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::O)) {
            self.choose_image(&ctx);
        }

        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    self.choose_image(&ctx);
                }
                if ui.button("−").clicked() {
                    self.zoom = (self.zoom / 1.2).max(0.1);
                }
                if ui.button("+").clicked() {
                    self.zoom = (self.zoom * 1.2).min(10.0);
                }
                if ui.button("Fit").clicked() {
                    self.zoom = 1.0;
                }
                if let Some(path) = &self.path {
                    ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }

            if let (Some(texture), Some(image_size)) = (&self.texture, self.image_size) {
                let available = ui.available_size();
                let fit_scale = (available.x / image_size.x)
                    .min(available.y / image_size.y)
                    .min(1.0);
                let display_size = image_size * (fit_scale * self.zoom);
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let canvas_size = ui.available_size().max(display_size);
                        ui.allocate_ui_with_layout(
                            canvas_size,
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
                            },
                        );
                    });
            } else if self.error.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("Open or drop a JPG or PNG image to get started.");
                });
            }
        });
    }
}

fn load_image(path: &Path) -> Result<egui::ColorImage, String> {
    if !is_supported_image(path) {
        return Err("Please select a JPG or PNG image.".to_owned());
    }

    let bytes = std::fs::read(path).map_err(|error| format!("Could not read image: {error}"))?;
    decode_image(&bytes).map_err(|error| format!("Could not open image: {error}"))
}

fn decode_image(bytes: &[u8]) -> image::ImageResult<egui::ColorImage> {
    let rgba = image::load_from_memory(bytes)?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png"
            )
        })
}

fn main() -> eframe::Result {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MatataView",
        options,
        Box::new(move |creation_context| {
            Ok(Box::new(MatataView::new(
                &creation_context.egui_ctx,
                initial_path,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    #[test]
    fn recognizes_jpg_and_png_extensions_case_insensitively() {
        assert!(is_supported_image(Path::new("photo.jpg")));
        assert!(is_supported_image(Path::new("photo.JPEG")));
        assert!(is_supported_image(Path::new("photo.PNG")));
        assert!(!is_supported_image(Path::new("photo.gif")));
        assert!(!is_supported_image(Path::new("photo")));
    }

    #[test]
    fn decodes_png_pixels_for_display() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, image::Rgb([12, 34, 56])));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();

        let decoded = decode_image(bytes.get_ref()).unwrap();

        assert_eq!(decoded.size, [2, 1]);
        assert_eq!(decoded.pixels[0], egui::Color32::from_rgb(12, 34, 56));
    }

    #[test]
    fn decodes_jpeg_dimensions_for_display() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(3, 2, image::Rgb([12, 34, 56])));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Jpeg).unwrap();

        let decoded = decode_image(bytes.get_ref()).unwrap();

        assert_eq!(decoded.size, [3, 2]);
    }
}
