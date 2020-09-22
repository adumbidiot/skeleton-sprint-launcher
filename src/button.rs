use crate::LauncherState;
use ggez::{
    graphics,
    graphics::{
        Color,
        Rect,
        Text,
    },
    Context,
    GameResult,
};

fn noop(_state: &LauncherState) {}

pub struct ButtonBuilder {
    text: Text,
    dim: Rect,
    key_up_handler: fn(&LauncherState),
}

impl ButtonBuilder {
    pub fn new() -> Self {
        ButtonBuilder {
            text: Text::new(""),
            dim: [0.0, 0.0, 1.0, 1.0].into(),
            key_up_handler: noop,
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = Text::new(text);
        self
    }

    pub fn dimensions<T: Into<Rect>>(mut self, rect: T) -> Self {
        self.dim = rect.into();
        self
    }

    pub fn key_up_handler(mut self, handler: fn(&LauncherState)) -> Self {
        self.key_up_handler = handler;
        self
    }

    pub fn build(self, ctx: &mut Context) -> GameResult<Button> {
        let draw_mode = graphics::DrawMode::Fill(Default::default());
        let radius = self.dim.h / 2.0;
        let tolerance = 0.1;
        let background = graphics::MeshBuilder::new()
            .circle(
                draw_mode,
                [radius, radius],
                radius,
                tolerance,
                graphics::WHITE,
            )
            .rectangle(
                draw_mode,
                [radius, 0.0, self.dim.w - (2.0 * radius), self.dim.h].into(),
                graphics::WHITE,
            )
            .circle(
                draw_mode,
                [self.dim.w - radius, radius],
                radius,
                tolerance,
                graphics::WHITE,
            )
            .build(ctx)?;

        Ok(Button {
            text: self.text,
            background,
            dim: self.dim,

            keydown: false,
            new_key_up: false,

            color: Color::from_rgb(255, 0, 0),

            normal_color: Color::from_rgb(255, 0, 0),
            clicked_color: Color::from_rgb(128, 0, 0),

            key_up_handler: self.key_up_handler,
        })
    }
}

/// I will make these the prettiest, most overengineeered piece of shit in this whole damn project and you don't have the balls to stop me
pub struct Button {
    text: Text,
    background: graphics::Mesh,
    dim: Rect,

    keydown: bool,

    new_key_up: bool,

    color: Color,

    normal_color: Color,
    clicked_color: Color,

    key_up_handler: fn(&LauncherState),
}

impl Button {
    pub fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        graphics::draw(
            ctx,
            &self.background,
            graphics::DrawParam::default()
                .dest(self.dim.point())
                .color(self.color),
        )?;

        let (w, h) = self.text.dimensions(ctx);
        let mut loc = self.dim.point();
        loc.x += (self.dim.w - w as f32) / 2.0;
        loc.y += (self.dim.h - h as f32) / 2.0;

        graphics::draw(ctx, &self.text, graphics::DrawParam::default().dest(loc))?;
        Ok(())
    }

    pub fn update(&mut self, _state: &LauncherState) {
        if self.keydown && self.color != self.clicked_color {
            self.color.r = self.clicked_color.r; // TODO: Transistions
            self.color.g = self.clicked_color.g; // TODO: Consider TARGET_COLOR vs ACTUAL_COLOR fields to make more generic
            self.color.b = self.clicked_color.b;
        } else if !self.keydown && self.color != self.normal_color {
            self.color.r = self.normal_color.r;
            self.color.g = self.normal_color.g;
            self.color.b = self.normal_color.b;
        }
    }

    pub fn on_keydown(&mut self, _state: &LauncherState) {
        //TODO: Accept coords? Accept keys?
        self.keydown = true;
    }

    pub fn on_keyup(&mut self, state: &LauncherState) {
        self.keydown = false;
        self.new_key_up = true;
        (self.key_up_handler)(state);
    }

    pub fn dimensions(&self) -> &Rect {
        &self.dim
    }
}
