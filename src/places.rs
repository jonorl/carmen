//! Little pixel-art pictures of the places you can investigate, shown in
//! the "where to?" picker. Each is drawn on a 104x68 canvas with a black
//! background, to sit on the black picker window.

use macroquad::prelude::*;

use crate::game::PLACES;
use crate::pixels::{Canvas, Rgb, text_width};

pub const ART_W: i32 = 104;
pub const ART_H: i32 = 68;

const WHITE: Rgb = [245, 245, 240];
const STONE: Rgb = [222, 208, 178];
const STONE_SHADE: Rgb = [172, 156, 128];
const SALMON: Rgb = [232, 150, 130];
const SALMON_DARK: Rgb = [184, 100, 86];
const BRICK: Rgb = [172, 82, 62];
const LIGHT_GREY: Rgb = [200, 200, 205];
const GREY: Rgb = [150, 150, 155];
const DARK: Rgb = [60, 60, 70];
const LIT: Rgb = [255, 240, 90];
const GLASS: Rgb = [110, 180, 225];
const RED: Rgb = [220, 50, 60];
const GREEN: Rgb = [40, 160, 70];
const BLUE: Rgb = [70, 90, 220];
const YELLOW: Rgb = [245, 200, 40];
const BROWN: Rgb = [120, 78, 44];

/// One picture per entry in PLACES, in the same order.
pub fn pictures() -> Vec<Texture2D> {
    PLACES
        .iter()
        .map(|place| {
            let mut cv = Canvas::new(ART_W, ART_H);
            match place.name {
                "Bank" => bank(&mut cv),
                "Airport" => airport(&mut cv),
                "Library" => library(&mut cv),
                "Museum" => museum(&mut cv),
                "Market" => market(&mut cv),
                "Hotel" => hotel(&mut cv),
                "Bus station" => bus_station(&mut cv),
                "Café" => cafe(&mut cv),
                "Taxi rank" => taxi_rank(&mut cv),
                _ => {}
            }
            cv.texture()
        })
        .collect()
}

/// A sign centred on cx.
fn sign(cv: &mut Canvas, cx: i32, y: i32, text: &str, c: Rgb) {
    cv.text(cx - text_width(text) / 2, y, text, c);
}

fn flag(cv: &mut Canvas, x: i32, top: i32, colours: [Rgb; 2]) {
    cv.rect(x, top, 1, 9, LIGHT_GREY);
    cv.rect(x + 1, top, 8, 3, colours[0]);
    cv.rect(x + 1, top + 3, 8, 2, colours[1]);
}

/// Striped awning with a scalloped edge.
fn awning(cv: &mut Canvas, x: i32, y: i32, w: i32, colour: Rgb) {
    for i in 0..w {
        let c = if (i / 4) % 2 == 0 { colour } else { WHITE };
        cv.rect(x + i, y, 1, 6, c);
        if i % 4 != 0 {
            cv.set(x + i, y + 6, c);
        }
    }
}

fn bank(cv: &mut Canvas) {
    // A classical temple front: steps, columns, a frieze with the name and a pediment.
    cv.rect(10, 63, 84, 3, GREY);
    cv.rect(14, 60, 76, 3, LIGHT_GREY);
    cv.rect(18, 56, 68, 4, STONE);
    cv.rect(48, 38, 9, 18, BROWN);
    // Three columns either side, leaving the doorway clear.
    for x in [20, 30, 40, 59, 69, 79] {
        cv.rect(x, 30, 5, 26, WHITE);
        cv.rect(x + 4, 30, 1, 26, STONE_SHADE);
        cv.rect(x - 1, 29, 7, 2, STONE);
    }
    cv.rect(16, 22, 72, 7, STONE);
    sign(cv, 52, 23, "BANK", DARK);
    cv.peak(52, 21, 38, 12, STONE);
    cv.peak(52, 19, 28, 7, STONE_SHADE);
    flag(cv, 52, 0, [GREEN, WHITE]);
}

fn airport(cv: &mut Canvas) {
    // A tail fin peeping over the terminal, a control tower and a windsock.
    for i in 0..14 {
        cv.rect(30 + i / 2, 43 - i, 9 - i / 2, 1, WHITE);
    }
    cv.rect(31, 36, 6, 2, RED);
    cv.rect(2, 41, 100, 3, DARK);
    cv.rect(4, 44, 96, 20, LIGHT_GREY);
    cv.rect(8, 47, 88, 8, GLASS);
    for x in (8..96).step_by(6) {
        cv.rect(x, 47, 1, 8, DARK);
    }
    cv.rect(46, 56, 10, 8, DARK);
    cv.rect(70, 14, 8, 27, GREY);
    cv.rect(77, 14, 1, 27, DARK);
    cv.rect(64, 6, 20, 9, DARK);
    cv.rect(65, 8, 18, 5, GLASS);
    cv.rect(66, 4, 16, 2, DARK);
    cv.rect(74, 0, 1, 4, LIGHT_GREY);
    cv.set(74, 0, RED);
    cv.rect(12, 24, 1, 17, LIGHT_GREY);
    for i in 0..7 {
        cv.rect(13 + i, 24 + i / 4, 1, 3 - i / 4, if (i / 2) % 2 == 0 { RED } else { WHITE });
    }
}

fn library(cv: &mut Canvas) {
    // Red brick with arched windows, a stone portico and the name on the cornice.
    cv.rect(10, 26, 84, 38, BRICK);
    for y in (28..64).step_by(4) {
        cv.rect(10, y, 84, 1, [140, 62, 48]);
    }
    for x in [16, 26, 72, 82] {
        cv.rect(x, 38, 7, 12, LIT);
        cv.disc(x + 3, 38, 3, LIT);
        cv.rect(x + 3, 35, 1, 15, DARK);
    }
    cv.rect(38, 30, 28, 34, STONE);
    for x in [40, 47, 55, 62] {
        cv.rect(x, 34, 3, 26, WHITE);
    }
    cv.rect(48, 46, 8, 18, BROWN);
    cv.rect(8, 18, 88, 8, STONE);
    sign(cv, 52, 19, "LIBRARY", DARK);
    cv.peak(52, 17, 18, 8, STONE);
    cv.rect(34, 62, 36, 2, GREY);
    cv.rect(30, 64, 44, 2, LIGHT_GREY);
}

fn museum(cv: &mut Canvas) {
    // Like the original's: battlemented wings either side of a grand arch.
    for x in [4, 74] {
        cv.rect(x, 32, 26, 32, SALMON);
        for i in 0..5 {
            cv.rect(x + i * 6, 28, 4, 4, SALMON);
        }
        cv.rect(x + 7, 40, 6, 9, LIT);
        cv.rect(x + 7, 40, 6, 1, DARK);
        cv.rect(x + 7, 54, 6, 9, LIT);
    }
    cv.rect(28, 24, 48, 40, [240, 176, 156]);
    cv.rect(26, 21, 52, 4, SALMON_DARK);
    cv.peak(52, 20, 24, 8, SALMON_DARK);
    cv.disc(52, 42, 12, DARK);
    cv.rect(40, 42, 25, 22, DARK);
    for x in (42..64).step_by(4) {
        cv.rect(x, 46, 2, 18, GREY);
    }
    cv.rect(26, 60, 52, 4, LIGHT_GREY);
    flag(cv, 51, 6, [RED, WHITE]);
}

fn market(cv: &mut Canvas) {
    // Three stalls with striped awnings and heaps of fruit, under a banner.
    cv.rect(28, 6, 48, 9, YELLOW);
    sign(cv, 52, 8, "MARKET", DARK);
    for (k, colour) in [RED, GREEN, BLUE].into_iter().enumerate() {
        let x = 3 + k as i32 * 34;
        cv.rect(x, 24, 2, 40, BROWN);
        cv.rect(x + 28, 24, 2, 40, BROWN);
        awning(cv, x - 1, 22, 32, colour);
        cv.rect(x + 1, 48, 28, 16, BROWN);
        cv.rect(x + 1, 48, 28, 1, [160, 110, 60]);
        let fruit: [Rgb; 3] = [[245, 140, 30], [210, 40, 40], [120, 200, 60]];
        for i in 0..7 {
            for j in 0..2 {
                cv.rect(x + 2 + i * 4, 43 + j * 2, 3, 2, fruit[((i + j) as usize + k) % 3]);
            }
        }
    }
}

fn hotel(cv: &mut Canvas) {
    // A tall block with rows of lit windows and a neon sign down the side.
    cv.rect(14, 30, 16, 34, STONE_SHADE);
    cv.rect(74, 30, 16, 34, STONE_SHADE);
    cv.rect(30, 6, 44, 58, [232, 222, 194]);
    for (row, y) in (10..48).step_by(7).enumerate() {
        for (col, x) in (34..70).step_by(8).enumerate() {
            let lit = (row * 3 + col) % 4 != 0;
            cv.rect(x, y, 5, 4, if lit { LIT } else { DARK });
        }
    }
    for y in (34..60).step_by(7) {
        cv.rect(17, y, 10, 4, GLASS);
        cv.rect(77, y, 10, 4, GLASS);
    }
    cv.rect(52, 0, 1, 6, LIGHT_GREY);
    cv.rect(78, 4, 9, 32, RED);
    for (i, ch) in "HOTEL".chars().enumerate() {
        cv.text(81, 6 + i as i32 * 6, &ch.to_string(), WHITE);
    }
    cv.rect(40, 52, 24, 3, RED);
    cv.rect(41, 55, 1, 9, LIGHT_GREY);
    cv.rect(62, 55, 1, 9, LIGHT_GREY);
    cv.rect(46, 55, 12, 9, GLASS);
}

fn bus_station(cv: &mut Canvas) {
    // A shelter, a bus pulled up in front and a bus-stop sign.
    cv.rect(4, 20, 92, 4, [30, 110, 60]);
    cv.rect(6, 24, 88, 36, [170, 200, 205]);
    for x in [6, 50, 92] {
        cv.rect(x, 24, 2, 40, DARK);
    }
    cv.rect(12, 34, 76, 24, YELLOW);
    cv.rect(12, 34, 76, 1, [255, 230, 120]);
    cv.rect(16, 38, 58, 9, GLASS);
    for x in (16..74).step_by(10) {
        cv.rect(x, 38, 1, 9, DARK);
    }
    cv.rect(77, 38, 7, 18, DARK);
    cv.rect(78, 39, 5, 8, GLASS);
    cv.rect(12, 50, 76, 2, [200, 140, 20]);
    for x in [26, 72] {
        cv.disc(x, 59, 5, DARK);
        cv.disc(x, 59, 2, GREY);
    }
    cv.rect(96, 8, 2, 56, LIGHT_GREY);
    cv.rect(89, 6, 15, 9, BLUE);
    sign(cv, 96, 8, "BUS", WHITE);
}

fn cafe(cv: &mut Canvas) {
    // A corner café: name board, striped awning, a big window and a table outside.
    cv.rect(8, 14, 80, 50, [214, 182, 148]);
    for x in [16, 40, 64] {
        cv.rect(x, 18, 10, 9, GLASS);
        cv.rect(x + 4, 18, 1, 9, WHITE);
    }
    cv.rect(30, 28, 36, 9, [30, 100, 60]);
    sign(cv, 48, 30, "CAFE", [250, 240, 210]);
    awning(cv, 6, 38, 84, RED);
    cv.rect(14, 47, 40, 15, GLASS);
    cv.rect(14, 47, 40, 1, WHITE);
    cv.rect(62, 46, 14, 18, BROWN);
    cv.set(73, 55, LIT);
    cv.peak(94, 46, 10, 6, RED);
    cv.rect(94, 46, 1, 14, LIGHT_GREY);
    cv.rect(89, 56, 11, 2, WHITE);
    cv.rect(90, 58, 1, 6, LIGHT_GREY);
    cv.rect(98, 58, 1, 6, LIGHT_GREY);
}

fn taxi_rank(cv: &mut Canvas) {
    // Two yellow cabs waiting by a TAXI sign.
    cv.rect(0, 64, 104, 3, GREY);
    cv.rect(12, 16, 2, 48, LIGHT_GREY);
    cv.rect(3, 6, 20, 10, YELLOW);
    sign(cv, 13, 8, "TAXI", DARK);
    for x in [20, 62] {
        cv.rect(x, 46, 40, 11, YELLOW);
        cv.rect(x + 8, 37, 23, 10, YELLOW);
        cv.rect(x + 10, 39, 9, 6, GLASS);
        cv.rect(x + 21, 39, 8, 6, GLASS);
        cv.rect(x + 15, 33, 9, 4, WHITE);
        for i in 0..20 {
            cv.rect(x + i * 2, 50 + (i % 2), 2, 1, DARK);
        }
        cv.rect(x + 38, 48, 2, 2, WHITE);
        cv.rect(x, 48, 1, 2, RED);
        for wx in [x + 9, x + 31] {
            cv.disc(wx, 58, 4, DARK);
            cv.disc(wx, 58, 1, GREY);
        }
    }
}
