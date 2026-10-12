//! The jail the arrested suspect is marched off to, as in the original: a
//! brick wall with a sign over a barred window, and the prisoner glaring
//! out from under their hat. Painted small and drawn at double size, so
//! its pixels are as chunky as the march's.

use macroquad::prelude::*;

use crate::pixels::{Canvas, Rgb, scale};

const W: i32 = 120;
const H: i32 = 116;
const SCALE: f32 = 2.0;

/// Where the prisoner is looking.
#[derive(Clone, Copy)]
pub enum Eyes {
    Left,
    Ahead,
    Right,
}

const RED_BRICK: Rgb = [0xb8, 0x5c, 0x54];
const PINK_BRICK: Rgb = [0xc4, 0x94, 0x94];
const BRICK_LIGHT: Rgb = [0xf4, 0xc0, 0x98];
const BRICK_SHADOW: Rgb = [0x6c, 0x34, 0x2c];
const SIGN: Rgb = [0xf8, 0xcc, 0xa0];
const LETTERS: Rgb = [0x8c, 0x4c, 0x3c];
const FRAME_DARK: Rgb = [0x3c, 0x3c, 0x3c];
const FRAME: Rgb = [0x8c, 0x8c, 0x8c];
const CELL: Rgb = [0xc8, 0xa4, 0xa4];
const CEILING: Rgb = [0x9c, 0x9c, 0x9c];
const BAR: Rgb = [0x68, 0x68, 0x68];
const BAR_LIGHT: Rgb = [0xa8, 0xa8, 0xa8];
const OUTLINE: Rgb = [0x28, 0x1c, 0x18];
const HAT: Rgb = [0x96, 0x52, 0x3c];
const HAT_BAND: Rgb = [0x60, 0x30, 0x24];
const COAT: Rgb = [0xf0, 0xcc, 0xa4];
const COAT_SEAM: Rgb = [0xa8, 0x7c, 0x5c];
const SKIN: Rgb = [0xec, 0xb0, 0x92];
const SKIN_SHADOW: Rgb = [0xc8, 0x8c, 0x6e];
const BRIGHT: Rgb = [0xff, 0xff, 0xff];

/// The sign's letters, 9 pixels tall with thick strokes.
const JAIL: [&[&str; 9]; 4] = [
    &[".#######", "...###..", "...###..", "...###..", "...###..", "...###..", "##.###..", "##.###..", ".####..."],
    &["...###...", "..#####..", "..##.##..", ".###.###.", ".##...##.", "#########", "###...###", "###...###", "###...###"],
    &["#######", "..###..", "..###..", "..###..", "..###..", "..###..", "..###..", "..###..", "#######"],
    &["#####....", ".###.....", ".###.....", ".###.....", ".###.....", ".###...##", ".###...##", ".###..###", "#########"],
];

/// A fist round a crossbar, centred on it.
const HAND: [&str; 6] = [".kkkkkk.", "kSSSSSSk", "kSsSsSsk", "kSsSsSsk", "kSSSSSSk", ".kkkkkk."];

/// Every version of the picture: each way the prisoner looks, with their
/// hands down and then on the bars.
pub struct Jail {
    pictures: Vec<Texture2D>,
}

impl Jail {
    pub fn new() -> Jail {
        let mut pictures = Vec::new();
        for grip in [false, true] {
            for eyes in [Eyes::Left, Eyes::Ahead, Eyes::Right] {
                pictures.push(paint(eyes, grip).texture());
            }
        }
        Jail { pictures }
    }

    /// Draw the picture centred near the top of r. Returns its bottom edge.
    pub fn draw(&self, r: Rect, eyes: Eyes, grip: bool) -> f32 {
        let (w, h) = (W as f32 * SCALE, H as f32 * SCALE);
        let (x, y) = ((r.x + (r.w - w) / 2.0).round(), r.y + 10.0);
        let picture = &self.pictures[grip as usize * 3 + eyes as usize];
        draw_texture_ex(picture, x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
        y + h
    }
}

fn paint(eyes: Eyes, grip: bool) -> Canvas {
    let mut cv = Canvas::new(W, H);
    wall(&mut cv);
    sign(&mut cv);
    window(&mut cv, eyes, grip);
    cv
}

/// Rows of bricks offset by half a brick, so both sides zigzag; the top row
/// is mostly behind the sign and the bottom one is short.
fn wall(cv: &mut Canvas) {
    for row in 0..14 {
        let y = 3 + row * 8;
        let shift = if row % 2 == 0 { 0 } else { 8 };
        let bricks = if row == 0 || row == 13 { 1..6 } else { 0..7 };
        for n in bricks {
            let x = shift + n * 16;
            let tone = if (row * 5 + n * 3 + row * n) % 7 < 3 { PINK_BRICK } else { RED_BRICK };
            brick(cv, x, y, tone);
        }
    }
}

fn brick(cv: &mut Canvas, x: i32, y: i32, tone: Rgb) {
    let (w, h) = (15, 7);
    cv.rect(x, y, w, h, tone);
    // Rough speckles, from a hash so every brick differs but never changes.
    let speck = scale(tone, 0.78);
    for py in y + 2..y + h - 1 {
        for px in x + 2..x + w - 1 {
            let n = (px as u32).wrapping_mul(0x9e37_79b1) ^ (py as u32).wrapping_mul(0x85eb_ca77);
            if (n >> 13).is_multiple_of(5) {
                cv.set(px, py, speck);
            }
        }
    }
    cv.rect(x, y, w, 1, BRICK_LIGHT);
    cv.rect(x, y, 1, h, BRICK_LIGHT);
    cv.rect(x, y + h - 1, w, 1, BRICK_SHADOW);
    cv.rect(x + w - 1, y + 1, 1, h - 1, BRICK_SHADOW);
}

fn sign(cv: &mut Canvas) {
    cv.rect(21, 0, 78, 19, OUTLINE);
    cv.rect(22, 1, 76, 17, SIGN);
    cv.rect(22, 16, 76, 2, scale(SIGN, 0.8));
    let mut x = 41;
    for glyph in JAIL {
        for (dy, row) in glyph.iter().enumerate() {
            for (dx, bit) in row.chars().enumerate() {
                if bit == '#' {
                    cv.set(x + dx as i32, 5 + dy as i32, LETTERS);
                }
            }
        }
        x += glyph[0].len() as i32 + 2;
    }
}

fn window(cv: &mut Canvas, eyes: Eyes, grip: bool) {
    // A heavy grey frame round the cell.
    cv.rect(37, 31, 46, 66, FRAME_DARK);
    cv.rect(38, 32, 44, 64, FRAME);
    cv.rect(41, 35, 38, 58, FRAME_DARK);
    cv.rect(42, 36, 36, 56, CELL);
    cv.rect(42, 36, 36, 7, CEILING);

    // A lamp hanging from the ceiling, lighting the back wall.
    let cx = 58;
    cv.set(cx, 36, OUTLINE);
    cv.rect(cx - 2, 37, 5, 1, OUTLINE);
    cv.rect(cx - 3, 38, 7, 2, OUTLINE);
    cv.rect(cx - 1, 40, 3, 1, [0xf0, 0xe8, 0x60]);
    cv.rect(cx - 4, 46, 9, 2, [0xf4, 0xec, 0xe8]);

    // Something on the wall at the back, as in the original.
    cv.rect(69, 64, 9, 18, OUTLINE);
    cv.rect(70, 65, 7, 16, FRAME);
    for x in [71, 73, 75] {
        cv.rect(x, 73, 1, 7, BRIGHT);
    }

    prisoner(cv, cx, eyes);

    // Bars in front, one down the middle of the face.
    for x in [cx - 12, cx - 1, cx + 10] {
        cv.rect(x, 36, 2, 56, BAR);
        cv.rect(x, 36, 1, 56, BAR_LIGHT);
    }
    for y in [43, 84] {
        cv.rect(42, y, 36, 2, BAR);
        cv.rect(42, y, 36, 1, BAR_LIGHT);
    }

    if grip {
        for x in [cx - 15, cx + 8] {
            sprite(cv, &HAND, x, 82);
        }
    }
}

/// The prisoner: hat brim low, face in shadow but for the eyes, coat
/// collar up.
fn prisoner(cv: &mut Canvas, cx: i32, eyes: Eyes) {
    for y in 74..92 {
        let half = 7 + (y - 74) * 7 / 17;
        cv.rect(cx - half - 1, y, 2 * half + 3, 1, OUTLINE);
        cv.rect(cx - half, y, 2 * half + 1, 1, COAT);
    }
    cv.line(cx - 1, 74, cx - 7, 88, COAT_SEAM);
    cv.line(cx + 1, 74, cx + 7, 88, COAT_SEAM);
    for x in [cx - 10, cx + 7] {
        cv.rect(x, 67, 4, 8, OUTLINE);
        cv.rect(x + 1, 68, 2, 7, COAT);
    }

    cv.rect(cx - 6, 67, 13, 8, [0x10, 0x08, 0x08]);
    let look = match eyes {
        Eyes::Left => 0,
        Eyes::Ahead => 1,
        Eyes::Right => 2,
    };
    for x in [cx - 5, cx + 2] {
        cv.rect(x, 69, 3, 2, BRIGHT);
        cv.set(x + look, 70, OUTLINE);
    }

    cv.rect(cx - 6, 58, 13, 7, OUTLINE);
    cv.rect(cx - 5, 59, 11, 4, HAT);
    cv.set(cx, 59, HAT_BAND); // the crease in the crown
    cv.rect(cx - 5, 63, 11, 2, HAT_BAND);
    cv.rect(cx - 12, 64, 25, 3, OUTLINE);
    cv.rect(cx - 11, 65, 23, 1, HAT);
}

fn sprite(cv: &mut Canvas, rows: &[&str], x: i32, y: i32) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let colour = match ch {
                'k' => OUTLINE,
                'S' => SKIN,
                's' => SKIN_SHADOW,
                _ => continue,
            };
            cv.set(x + dx as i32, y + dy as i32, colour);
        }
    }
}
