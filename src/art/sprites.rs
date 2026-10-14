//! The little characters of the chase, the walk to jail, the V.I.L.E.
//! henchmen and the final city's warnings, traced pixel for pixel from
//! recordings of the original game.
//! Each is a strip of same-size frames in assets/sprites/ (black is
//! see-through), drawn at double size, which suits a text panel about twice
//! the original's, and stepping along the bottom of the panel at the
//! original's pace rather than gliding.

use macroquad::prelude::*;

const SCALE: f32 = 2.0;

/// One character's animation.
pub struct Anim {
    strip: Texture2D,
    /// One frame's size, in the original's pixels.
    w: f32,
    h: f32,
    /// Ticks a second, and the original pixels moved each tick (negative is
    /// leftwards).
    rate: f64,
    step: f32,
    /// The frame shown on each tick of the cycle.
    cycle: &'static [usize],
}

impl Anim {
    fn new(png: &[u8], (w, h): (f32, f32), rate: f64, step: f32, cycle: &'static [usize]) -> Anim {
        let strip = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
        strip.set_filter(FilterMode::Nearest);
        Anim { strip, w, h, rate, step, cycle }
    }

    /// How long it takes to cross `panel` and be gone.
    pub fn seconds(&self, panel: Rect) -> f64 {
        ((panel.w + self.w * SCALE) / (self.step.abs() * SCALE)).ceil() as f64 / self.rate
    }

    /// Draw it `t` seconds after it set off from just outside `panel`,
    /// standing on the panel's bottom edge and clipped to its sides.
    pub fn draw(&self, panel: Rect, t: f64) {
        if t < 0.0 {
            return;
        }
        let tick = (t * self.rate) as usize;
        let (w, h) = (self.w * SCALE, self.h * SCALE);
        // The first tick already shows one step of it.
        let moved = self.step * SCALE * (tick + 1) as f32;
        let x = if self.step > 0.0 { panel.x - w + moved } else { panel.x + panel.w + moved };
        let (left, right) = (x.max(panel.x), (x + w).min(panel.x + panel.w));
        if right <= left {
            return;
        }
        let frame = self.cycle[tick % self.cycle.len()] as f32;
        let source = Rect::new(frame * self.w + (left - x) / SCALE, 0.0, (right - left) / SCALE, self.h);
        let params = DrawTextureParams { dest_size: Some(vec2(right - left, h)), source: Some(source), ..Default::default() };
        draw_texture_ex(&self.strip, left, panel.y + panel.h - h, WHITE, params);
    }
}

/// The henchman who pops up from the bottom of the panel, as in the
/// original: he rises a pixel every thirtieth of a second, looks about, and
/// sinks back down. Frames: facing front, looking right, looking left,
/// grinning.
pub struct PopUp {
    strip: Texture2D,
}

const POP_W: f32 = 42.0;
const POP_H: f32 = 32.0;
const POP_RISE: f64 = POP_H as f64 / 30.0;
const POP_HOLD: f64 = 2.53;
/// While he's up: from when, which frame.
const POP_LOOKS: [(f64, usize); 6] = [(0.0, 0), (0.5, 1), (1.0, 0), (1.5, 2), (2.0, 3), (2.25, 0)];

impl PopUp {
    pub const SECONDS: f64 = POP_RISE + POP_HOLD + POP_RISE;

    /// Draw him `t` seconds after he starts rising, in the middle of the
    /// bottom edge of `panel`.
    pub fn draw(&self, panel: Rect, t: f64) {
        let (rows, frame) = if !(0.0..Self::SECONDS).contains(&t) {
            return;
        } else if t < POP_RISE {
            ((t * 30.0) as f32, 0)
        } else if t < POP_RISE + POP_HOLD {
            let up = t - POP_RISE;
            (POP_H, POP_LOOKS.iter().rev().find(|(at, _)| up >= *at).map_or(0, |&(_, f)| f))
        } else {
            (POP_H - ((t - POP_RISE - POP_HOLD) * 30.0) as f32, 0)
        };
        if rows < 1.0 {
            return;
        }
        // Only his top `rows` rows are above the bottom edge.
        let x = (panel.x + (panel.w - POP_W * SCALE) / 2.0).round();
        let params = DrawTextureParams {
            dest_size: Some(vec2(POP_W * SCALE, rows * SCALE)),
            source: Some(Rect::new(frame as f32 * POP_W, 0.0, POP_W, rows)),
            ..Default::default()
        };
        draw_texture_ex(&self.strip, x, panel.y + panel.h - rows * SCALE, WHITE, params);
    }
}

/// The final city's warnings, as in the original: an axe spinning in from
/// the left to bury itself in the right-hand edge, a knife thrown in from the
/// right into the left-hand edge, and a hand with a pistol sliding in from
/// the left, cocking it and firing a shot across.
pub struct Danger {
    axe: Texture2D,
    knife: Texture2D,
    gun: Texture2D,
}

fn strip(png: &[u8]) -> Texture2D {
    let strip = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
    strip.set_filter(FilterMode::Nearest);
    strip
}

/// Draw frame `frame` of `strip` (frames `w` by `h` original pixels) with
/// its top left at (x, y), clipped to the sides of `panel`.
fn draw_clipped(strip: &Texture2D, frame: usize, (w, h): (f32, f32), x: f32, y: f32, panel: Rect) {
    let (left, right) = (x.max(panel.x), (x + w * SCALE).min(panel.x + panel.w));
    if right <= left {
        return;
    }
    let source = Rect::new(frame as f32 * w + (left - x) / SCALE, 0.0, (right - left) / SCALE, h);
    let params = DrawTextureParams { dest_size: Some(vec2(right - left, h * SCALE)), source: Some(source), ..Default::default() };
    draw_texture_ex(strip, left, y, WHITE, params);
}

impl Danger {
    /// Draw warning `kind` (0 axe, 1 knife, 2 pistol) `t` seconds after it
    /// first shows. It stays put once it has landed or fired.
    pub fn draw(&self, kind: usize, panel: Rect, t: f64) {
        if t < 0.0 {
            return;
        }
        let bottom = panel.y + panel.h;
        match kind {
            0 => {
                // Spinning a quarter turn every 4 ticks, 6 pixels a tick at 30 a
                // second, until it hits the right-hand edge and sticks.
                let tick = (t * 30.0) as usize;
                let x = panel.x - 44.0 * SCALE + 6.0 * SCALE * (tick + 1) as f32;
                let stuck = panel.x + panel.w - 44.0 * SCALE;
                let (x, frame) = if x >= stuck { (stuck, 4) } else { (x, tick / 4 % 4) };
                draw_clipped(&self.axe, frame, (44.0, 43.0), x, bottom - (7.0 + 43.0) * SCALE, panel);
            }
            1 => {
                // Six pixels every sixtieth of a second, until its tip is 5
                // pixels into the left-hand edge.
                let moved = 6.0 * SCALE * ((t * 60.0) as usize + 1) as f32;
                let x = (panel.x + panel.w - moved).max(panel.x - 5.0 * SCALE);
                draw_clipped(&self.knife, 0, (59.0, 14.0), x, bottom - (27.0 + 14.0) * SCALE, panel);
            }
            _ => {
                // In at 2 pixels a sixtieth of a second, then pulled back, raised
                // and back again, then the shot.
                let slid = (-49.0 + 2.0 * (t * 60.0) as usize as f32).min(1.0);
                let frame = match t {
                    t if (0.533..0.633).contains(&t) || (0.733..0.833).contains(&t) => 1,
                    t if (0.633..0.733).contains(&t) => 2,
                    _ => 0,
                };
                draw_clipped(&self.gun, frame, (51.0, 27.0), panel.x + slid * SCALE, bottom - (14.0 + 27.0) * SCALE, panel);
                let bullet = panel.x + (50.0 + 240.0 * (t - 0.933) as f32) * SCALE;
                if t >= 0.933 && bullet < panel.x + panel.w {
                    let w = (6.0 * SCALE).min(panel.x + panel.w - bullet);
                    draw_rectangle(bullet, bottom - 34.0 * SCALE, w, SCALE, Color::from_rgba(239, 243, 243, 255));
                }
            }
        }
    }
}

/// Everyone in the chase, the walk to jail and the henchmen's appearances.
pub struct Cast {
    /// Running off to the right: eight frames, 4 pixels a tick, 15 ticks a second.
    pub suspect: Anim,
    /// Three officers giving chase: five frames, 4 pixels a tick, 20 a second.
    pub police: Anim,
    /// The suspect, hands up, walked off to the left at gunpoint: six frames,
    /// 2 pixels a tick, 12 a second.
    pub march: Anim,
    /// The officer trudging off alone when the suspect got away: ten poses
    /// held two ticks each, 2 pixels a tick, 15 a second.
    pub slumped: Anim,
    /// A henchman striding across to the right, waving: eight frames,
    /// 2 pixels a tick, 15 a second.
    pub stride: Anim,
    /// A henchman sneaking past to the left with a sack: eight poses held two
    /// ticks each, 2 pixels a tick, 20 a second.
    pub sneak: Anim,
    pub popup: PopUp,
    pub danger: Danger,
}

impl Cast {
    pub fn new() -> Cast {
        Cast {
            suspect: Anim::new(include_bytes!("../../assets/sprites/suspect.png"), (47.0, 50.0), 15.0, 4.0, &[0, 1, 2, 3, 4, 5, 6, 7]),
            police: Anim::new(include_bytes!("../../assets/sprites/police.png"), (49.0, 49.0), 20.0, 4.0, &[0, 1, 2, 3, 4]),
            march: Anim::new(include_bytes!("../../assets/sprites/march.png"), (49.0, 49.0), 12.0, -2.0, &[0, 1, 2, 3, 4, 5]),
            slumped: Anim::new(
                include_bytes!("../../assets/sprites/slumped.png"),
                (39.0, 45.0),
                15.0,
                -2.0,
                &[0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9],
            ),
            stride: Anim::new(
                include_bytes!("../../assets/sprites/henchman-stride.png"),
                (46.0, 42.0),
                15.0,
                2.0,
                &[0, 1, 2, 3, 4, 5, 6, 7],
            ),
            sneak: Anim::new(
                include_bytes!("../../assets/sprites/henchman-sneak.png"),
                (45.0, 41.0),
                20.0,
                -2.0,
                &[0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7],
            ),
            popup: PopUp { strip: strip(include_bytes!("../../assets/sprites/henchman-popup.png")) },
            danger: Danger {
                axe: strip(include_bytes!("../../assets/sprites/danger-axe.png")),
                knife: strip(include_bytes!("../../assets/sprites/danger-knife.png")),
                gun: strip(include_bytes!("../../assets/sprites/danger-gun.png")),
            },
        }
    }
}
