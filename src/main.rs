mod config;
pub mod steamworks_util;
mod ui;
mod util;

use crate::ui::App;
use conrod_core::{
    text::Font,
    Theme,
    UiBuilder,
};
use glutin::Icon;
use glutin_window::GlutinWindow;
use image::GenericImageView;
use piston_window::{
    texture::UpdateTexture,
    EventLoop,
    G2d,
    G2dTexture,
    OpenGL,
    PistonWindow,
    Texture,
    TextureSettings,
    UpdateEvent,
    Window,
    WindowSettings,
};
use std::error::Error as StdError;
use winit::BadIcon;

const COVER_IMAGE_DATA: &[u8] = include_bytes!("../assets/cover.png");
const FONT_DATA: &[u8] = include_bytes!("../assets/fonts/bolonewt/bolonewt.ttf");
const ICON_DATA: &[u8] = include_bytes!("../assets/icon.ico");

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

const WINDOW_TITLE: &str = "Skeleton Sprint Launcher";

#[derive(Debug)]
pub enum AssetError {
    Image(image::ImageError),
    BadIcon(BadIcon),
}

impl From<image::ImageError> for AssetError {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e)
    }
}

impl From<BadIcon> for AssetError {
    fn from(e: BadIcon) -> Self {
        Self::BadIcon(e)
    }
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image(e) => e.fmt(f),
            Self::BadIcon(e) => e.fmt(f),
        }
    }
}

impl StdError for AssetError {}

/// Loads an image into a winit icon
fn load_icon(icon_image_bytes: &[u8]) -> Result<Icon, AssetError> {
    let icon_image = image::load_from_memory(icon_image_bytes)?;
    let icon_width = icon_image.width();
    let icon_height = icon_image.height();
    let icon_rgba_buffer = icon_image.into_rgba8().into_vec();
    let icon = Icon::from_rgba(icon_rgba_buffer, icon_width, icon_height)?;
    Ok(icon)
}

/// Makes a PistonWindow
/// TODO: Add more configurable options and make more generic
fn make_piston_window(icon: Icon) -> Result<PistonWindow, Box<dyn StdError>> {
    let title = WINDOW_TITLE;
    let width = WINDOW_WIDTH;
    let height = WINDOW_HEIGHT;
    let num_samples = 4;
    let use_vsync = true;
    let resizable = false;
    let is_visible = false;

    // TODO: Is this multisamlping or antialiasing?
    let samples = 0;

    let window_settings = WindowSettings::new(title, [width, height])
        .resizable(resizable)
        .vsync(use_vsync)
        .samples(num_samples);

    let events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_resizable(resizable)
        .with_title(title)
        .with_visibility(is_visible)
        .with_dimensions((width, height).into())
        .with_window_icon(Some(icon))
        .with_multitouch();

    let glutin_window = GlutinWindow::from_raw(&window_settings, events_loop, window_builder)?;
    let mut window = PistonWindow::new(OpenGL::V3_2, samples, glutin_window);

    // Set eventloop updates
    window.events.set_ups(60);

    Ok(window)
}

fn main() {
    let font = match Font::from_bytes(FONT_DATA) {
        Ok(font) => font,
        Err(e) => {
            eprintln!("Failed to load font: {}", e);
            return;
        }
    };

    let icon = match load_icon(ICON_DATA) {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("Failed to load icon: {}", e);
            return;
        }
    };

    let cover_image = match image::load_from_memory(COVER_IMAGE_DATA) {
        Ok(cover_image) => cover_image.into_rgba8(),
        Err(e) => {
            eprintln!("Failed to load cover image: {}", e);
            return;
        }
    };

    let config = match crate::config::load_from_file("./config.toml") {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };

    let mut window = match make_piston_window(icon) {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Failed to make a window: {}", e);
            return;
        }
    };

    let mut texture_context = window.create_texture_context();
    let texture_settings = TextureSettings::new();

    let cover_image =
        match Texture::from_image(&mut texture_context, &cover_image, &texture_settings) {
            Ok(cover_image) => cover_image,
            Err(e) => {
                eprintln!("Failed to load cover_image into a texture: {}", e);
                return;
            }
        };

    let mut image_map = conrod_core::image::Map::new();
    let cover_image = image_map.insert(cover_image);

    let mut app = match App::new(config, cover_image) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to init app: {}", e);
            return;
        }
    };

    let mut ui = UiBuilder::new([WINDOW_WIDTH.into(), WINDOW_HEIGHT.into()])
        .theme(Theme::default())
        .build();
    ui.clear_with(conrod_core::color::Color::Rgba(0.0, 0.0, 0.0, 1.0));
    // ui.set_num_redraw_frames(10);

    let ids = self::ui::Ids::new(ui.widget_id_generator());
    ui.fonts.insert(font);

    let mut text_vertex_data = Vec::new();
    let (mut glyph_cache, mut text_texture_cache) = {
        const SCALE_TOLERANCE: f32 = 0.1;
        const POSITION_TOLERANCE: f32 = 0.1;

        let cache = conrod_core::text::GlyphCache::builder()
            .dimensions(WINDOW_WIDTH, WINDOW_HEIGHT)
            .scale_tolerance(SCALE_TOLERANCE)
            .position_tolerance(POSITION_TOLERANCE)
            .build();

        let buffer_len = WINDOW_WIDTH as usize * WINDOW_HEIGHT as usize;
        let init = vec![128; buffer_len];

        let settings = TextureSettings::new();
        let texture = G2dTexture::from_memory_alpha(
            &mut texture_context,
            &init,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            &settings,
        )
        .expect("Valid texture");

        (cache, texture)
    };

    // Don't show the window until the first draw is complete.
    let mut first_draw = true;
    let mut opened_window = false;

    while let Some(event) = window.next() {
        let size = window.size();
        let (win_w, win_h) = (
            size.width as conrod_core::Scalar,
            size.height as conrod_core::Scalar,
        );
        if let Some(e) = conrod_piston::event::convert(event.clone(), win_w, win_h) {
            ui.handle_event(e);
        }

        event.update(|_| {
            app.update();

            let mut ui = ui.set_widgets();
            ui::gui(&mut ui, &ids, &mut app);

            if !first_draw && !opened_window {
                let window = window.window.ctx.window();
                window.show();
                opened_window = true;
            }
        });

        window.draw_2d(&event, |context, graphics, device| {
            // if let Some(primitives) = ui.draw_if_changed() {
            // Force a draw here to get steam overlay to work right
            if let Some(primitives) = Some(ui.draw()) {
                let cache_queued_glyphs = |_graphics: &mut G2d,
                                           cache: &mut G2dTexture,
                                           rect: conrod_core::text::rt::Rect<u32>,
                                           data: &[u8]| {
                    let offset = [rect.min.x, rect.min.y];
                    let size = [rect.width(), rect.height()];
                    let format = piston_window::texture::Format::Rgba8;
                    text_vertex_data.clear();
                    text_vertex_data.extend(data.iter().flat_map(|&b| vec![255, 255, 255, b]));
                    UpdateTexture::update(
                        cache,
                        &mut texture_context,
                        format,
                        &text_vertex_data[..],
                        offset,
                        size,
                    )
                    .expect("failed to update texture")
                };

                conrod_piston::draw::primitives(
                    primitives,
                    context,
                    graphics,
                    &mut text_texture_cache,
                    &mut glyph_cache,
                    &image_map,
                    cache_queued_glyphs,
                    texture_from_image,
                );

                texture_context.encoder.flush(device);

                if first_draw {
                    first_draw = false;
                }
            }
        });
    }
}

fn texture_from_image<T>(img: &T) -> &T {
    img
}
