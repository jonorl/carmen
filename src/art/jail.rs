//! The jail the arrested suspect is marched off to, traced from recordings
//! of the original game: a brick wall with a sign over a barred window, the
//! prisoner glaring out from under their hat, then gripping the bars, or
//! nobody if they got away. Three frames in assets/sprites/jail.png, drawn at
//! double size like the walkers.

use macroquad::prelude::*;

const W: f32 = 126.0;
const H: f32 = 121.0;
const SCALE: f32 = 2.0;

pub struct Jail {
    strip: Texture2D,
}

impl Jail {
    pub fn new() -> Jail {
        let strip = Texture2D::from_file_with_format(include_bytes!("../../assets/sprites/jail.png"), Some(ImageFormat::Png));
        strip.set_filter(FilterMode::Nearest);
        Jail { strip }
    }

    /// Draw the picture centred at the top of r: the prisoner, gripping the
    /// bars once `Some(true)`, or an empty cell for None.
    pub fn draw(&self, r: Rect, prisoner: Option<bool>) {
        let frame = match prisoner {
            Some(false) => 0.0,
            Some(true) => 1.0,
            None => 2.0,
        };
        let (w, h) = (W * SCALE, H * SCALE);
        let x = (r.x + (r.w - w) / 2.0).round();
        let params = DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            source: Some(Rect::new(frame * W, 0.0, W, H)),
            ..Default::default()
        };
        draw_texture_ex(&self.strip, x, r.y + 4.0, WHITE, params);
    }
}
