//! Little running characters for the chase before an arrest, drawn as pixel
//! grids: each letter is a colour and '.' is see-through. They're drawn at
//! triple size, like the original's chunky sprites, and clipped to a panel
//! so they run in from behind its frame.

use macroquad::prelude::*;

use crate::ui::hex;

const SCALE: f32 = 3.0;

fn colour(c: char) -> Option<Color> {
    Some(match c {
        'k' => hex(0x281c18), // outline
        'H' => hex(0x96523c), // hat
        'h' => hex(0x603024), // hat band
        'C' => hex(0xf0cca4), // coat
        'c' => hex(0xcea47c), // coat seams
        'S' => hex(0xecb092), // skin
        's' => hex(0xc88c6e), // skin in shadow
        'G' => hex(0x9698a4), // shoes
        'B' => hex(0x5060dc), // uniform
        'b' => hex(0x343ea0), // belt
        'Y' => hex(0xfadc3c), // badge
        't' => hex(0xaaaab4), // truncheon
        'm' => hex(0x962828), // mouth
        _ => return None,
    })
}

/// Draw `rows` with its top-left at (x, y), keeping inside `clip`.
fn draw(rows: &[&str], x: f32, y: f32, clip: Rect) {
    for (ry, row) in rows.iter().enumerate() {
        for (rx, ch) in row.chars().enumerate() {
            let Some(c) = colour(ch) else { continue };
            let (px, py) = (x + rx as f32 * SCALE, y + ry as f32 * SCALE);
            if px >= clip.x && px + SCALE <= clip.x + clip.w && py >= clip.y && py + SCALE <= clip.y + clip.h {
                draw_rectangle(px, py, SCALE, SCALE, c);
            }
        }
    }
}

// The suspect: trench coat with the collar up, fedora, one fist forward.
const SUSPECT: [&str; 18] = [
    "........kkkkk.......",
    ".......kHHHHHk......",
    ".......kHHHHHk......",
    "......khhhhhhhk.....",
    "....kkHHHHHHHHHkk...",
    ".....kkCCCkSSSkk....",
    ".....kCCCCCksSSSk...",
    "....kCCCCCCCkkkk....",
    "....kCCCCCCCCk.kSk..",
    "...kCcCCCCCCCkkSSk..",
    "...kCCcCCCCCCCCkk...",
    "...kCCcCCCCCCCCk....",
    "...kCCCcCCCCCCCk....",
    "...kCCCCcCCCCCCk....",
    "..kCCCCCCCCCCCCk....",
    "..kCCCCCCCCCCCCCk...",
    "..kCCCCCCCCCCCCCk...",
    "..kkkkkkkkkkkkkkk...",
];
const SUSPECT_LEGS: [[&str; 6]; 2] = [
    [
        "....kCCk....kCCk....",
        "...kCCk......kCCk...",
        "..kCCk........kCCk..",
        ".kCCk..........kCCk.",
        "kGGGk..........kGGGG",
        "kkkk............kkkk",
    ],
    [
        "......kCCkkCCk......",
        "......kCCk.kCCk.....",
        ".....kCCk...kCCk....",
        ".....kCCk...kCCk....",
        "....kGGGGk..kGGGGk..",
        ".....kkkk....kkkk...",
    ],
];

// A police officer: domed helmet with a badge, big nose, truncheon raised.
const COP: [&str; 16] = [
    ".....kkkkk......",
    "....kBBBBBk.....",
    "...kBBBYBBBk....",
    "...kBBBBBBBk....",
    "..kkkkkkkkkkk...",
    "...kBkSSSSk.....",
    "...kkSSkSSSSk...",
    "....kSSSSSSSSk..",
    "....kSSmmSSkk...",
    ".....kSmmSk..kt.",
    "...kkbkkkkbk.kt.",
    "..kBBBBBBBBBkSSk",
    "..kBBBYBBBBkkSk.",
    "..kBBBBBBBBk.k..",
    "..kBBBBBBBBk....",
    "..kbbbbbbbbk....",
];
const COP_LEGS: [[&str; 5]; 2] = [
    [
        "...kBBk..kBBk...",
        "..kBBk....kBBk..",
        ".kBBk......kBBk.",
        "kGGGk......kGGGG",
        "kkkk........kkkk",
    ],
    [
        "....kBBkBBk.....",
        "....kBBkkBBk....",
        "...kBBk..kBBk...",
        "..kGGGGk.kGGGGk.",
        "...kkkk...kkkk..",
    ],
];

pub const SUSPECT_W: f32 = 20.0 * SCALE;
pub const POLICE_W: f32 = 38.0 * SCALE;

/// The suspect running right, feet on `ground`; `step` alternates the legs.
pub fn suspect(x: f32, ground: f32, step: usize, clip: Rect) {
    let top = ground - (SUSPECT.len() + 6) as f32 * SCALE;
    draw(&SUSPECT, x, top, clip);
    draw(&SUSPECT_LEGS[step % 2], x, top + SUSPECT.len() as f32 * SCALE, clip);
}

/// Three officers running right in a bunch, the one behind a little higher,
/// their legs out of step with each other.
pub fn police(x: f32, ground: f32, step: usize, clip: Rect) {
    let height = (COP.len() + 5) as f32 * SCALE;
    for (dx, lift, phase) in [(11.0, 8.0, 1), (0.0, 0.0, 0), (22.0, 0.0, 1)] {
        let (cx, top) = (x + dx * SCALE, ground - height - lift * SCALE / 2.0);
        draw(&COP, cx, top, clip);
        draw(&COP_LEGS[(step + phase) % 2], cx, top + COP.len() as f32 * SCALE, clip);
    }
}
