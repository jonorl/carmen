//! The crime computer as the original drew it: a beige CRT monitor on a
//! stand, sitting on a desktop case with two drive bays. The menu is drawn
//! on its screen by the caller.

use macroquad::prelude::*;

use crate::ui::{INK, fill, hex, outline, rounded};

const BEIGE: Color = hex(0xdfd2bc);
const BEIGE_LIGHT: Color = hex(0xf2eadc);
const BEIGE_DARK: Color = hex(0xb8a88e);
const BEIGE_SHADE: Color = hex(0xcbbca2);
const SLOT: Color = hex(0x2a2622);

/// The 16 EGA colours the lights flash through while computing.
const EGA: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555, 0x5555ff, 0x55ff55,
    0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

/// Light `i`'s colour `t` seconds into computing. Each light changes at its
/// own pace, to a colour picked by scrambling the light and the moment.
fn flashing(i: usize, t: f64) -> Color {
    let rate = [7.0, 9.5, 5.5][i % 3];
    let n = (t * rate + i as f64 * 0.37) as u64;
    // SplitMix64's finaliser: every bit of the input stirs every bit out.
    let mut z = (n * 3 + i as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    hex(EGA[(z % 16) as usize])
}

/// Draw the computer filling r (the text panel's inside) and return the
/// screen area, for the menu. `computing` is how long it has been working
/// on a warrant, if it is: the lights on its stand flash meanwhile.
pub fn draw(r: Rect, computing: Option<f64>) -> Rect {
    let at = |x: f32, y: f32, w: f32, h: f32| Rect::new(r.x + x, r.y + y, w, h);

    // The monitor: a rounded beige bezel around a recessed, dark screen.
    // Tall enough for nine rows of the big font.
    let bezel = at(8.0, 0.0, 308.0, 182.0);
    rounded(Rect::new(bezel.x - 1.0, bezel.y - 1.0, bezel.w + 2.0, bezel.h + 2.0), INK);
    rounded(bezel, BEIGE);
    fill(at(14.0, 2.0, 296.0, 1.0), BEIGE_LIGHT);
    rounded(at(15.0, 6.0, 294.0, 170.0), BEIGE_DARK);
    rounded(at(19.0, 9.0, 286.0, 164.0), hex(0x4a4a4a));
    let screen = at(22.0, 12.0, 280.0, 158.0);
    fill(screen, INK);

    // The stand, with vents either side of a panel of blue lights.
    let stand = at(64.0, 182.0, 196.0, 12.0);
    fill(stand, BEIGE_DARK);
    outline(stand, INK);
    for i in 0..5 {
        fill(at(74.0 + i as f32 * 7.0, 185.0, 2.0, 6.0), SLOT);
        fill(at(214.0 + i as f32 * 7.0, 185.0, 2.0, 6.0), SLOT);
    }
    let lights = at(114.0, 184.0, 96.0, 8.0);
    fill(lights, hex(0x1a1a2a));
    outline(lights, INK);
    for i in 0..3 {
        let colour = computing.map_or(hex(0x3c5aa8), |t| flashing(i, t));
        fill(at(120.0 + i as f32 * 29.0, 186.0, 24.0, 4.0), colour);
    }

    // The desktop case with two drive bays and a label plate.
    let case = at(20.0, 194.0, 284.0, 34.0);
    fill(case, BEIGE);
    outline(case, INK);
    fill(at(21.0, 195.0, 282.0, 1.0), BEIGE_LIGHT);
    outline(at(66.0, 206.0, 80.0, 12.0), BEIGE_DARK);
    for y in [197.0, 212.0] {
        let bay = at(186.0, y, 112.0, 13.0);
        fill(bay, BEIGE_SHADE);
        outline(bay, INK);
        fill(at(196.0, y + 4.0, 92.0, 5.0), SLOT);
    }

    // The base, its vents, and a cable trailing off to the keyboard.
    let base = at(6.0, 228.0, 312.0, 34.0);
    fill(base, BEIGE_SHADE);
    outline(base, INK);
    fill(at(7.0, 229.0, 310.0, 1.0), BEIGE_LIGHT);
    for i in 0..26 {
        fill(at(16.0 + i as f32 * 6.5, 236.0, 2.0, 20.0), SLOT);
    }
    outline(at(196.0, 232.0, 116.0, 26.0), INK);
    draw_line(r.x + 2.0, r.y + 216.0, r.x + 14.0, r.y + 202.0, 3.0, hex(0x5a7ab0));
    fill(at(12.0, 198.0, 6.0, 8.0), hex(0x8aa6c8));

    screen
}
