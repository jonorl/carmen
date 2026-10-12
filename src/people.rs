//! Pixel-art portraits of the witnesses, shown beside their speech bubble.
//! Each one is described as a `Look` (skin, hair, hat, glasses, beard,
//! outfit) and painted from a shared head-and-shoulders template, so a new
//! witness only needs a new entry in `look`.

use macroquad::prelude::*;

use crate::data::Suspect;
use crate::game::PLACES;
use crate::pixels::{Canvas, Rgb, dither, hash, scale};
use crate::rng::Rng;

pub const PORTRAIT_W: i32 = 66;
pub const PORTRAIT_H: i32 = 96;

const CX: i32 = 33; // the middle of the face
const OUTLINE: Rgb = [28, 22, 26];
const WHITE: Rgb = [240, 240, 236];
const GOLD: Rgb = [250, 210, 60];

#[derive(Clone, Copy)]
enum Hair {
    Short,
    Curly,
    Bun,
    Long,
    Bald,
}

#[derive(Clone, Copy)]
enum Hat {
    None,
    /// A guard's peaked cap with a badge.
    Peaked(Rgb),
    Flat(Rgb),
    Checked,
    /// A bank teller's green eyeshade.
    Visor,
    Pillbox(Rgb),
    Beanie(Rgb),
}

#[derive(Clone, Copy, PartialEq)]
enum Glasses {
    None,
    Round,
    Dark,
}

#[derive(Clone, Copy, PartialEq)]
enum Beard {
    None,
    Moustache,
    Full,
}

#[derive(Clone, Copy)]
enum Top {
    /// Shirt and tie; the colour is the tie.
    ShirtTie(Rgb),
    /// A jacket over a white shirt, with a gold badge if true.
    Jacket(bool),
    /// A uniform with a neckerchief in this colour.
    Scarf(Rgb),
    Cardigan,
    /// A shirt with an apron in this colour over it.
    Apron(Rgb),
    HiVis,
    BowTie,
}

struct Look {
    skin: Rgb,
    hair: Rgb,
    hair_style: Hair,
    hat: Hat,
    glasses: Glasses,
    beard: Beard,
    clothes: Rgb,
    top: Top,
}

const PALE: Rgb = [246, 206, 180];
const ROSY: Rgb = [236, 182, 160];
const OLIVE: Rgb = [214, 168, 124];
const TAN: Rgb = [196, 140, 100];
const BROWN: Rgb = [150, 98, 66];
const DEEP: Rgb = [104, 66, 46];

fn look(witness: &str) -> Look {
    let base = Look {
        skin: PALE,
        hair: [90, 60, 40],
        hair_style: Hair::Short,
        hat: Hat::None,
        glasses: Glasses::None,
        beard: Beard::None,
        clothes: [80, 80, 90],
        top: Top::Jacket(false),
    };
    match witness {
        "Bank teller" => Look {
            skin: ROSY,
            hair: [120, 80, 40],
            hair_style: Hair::Bald,
            hat: Hat::Visor,
            glasses: Glasses::Round,
            clothes: WHITE,
            top: Top::ShirtTie([180, 40, 50]),
            ..base
        },
        "Check-in agent" => Look {
            skin: TAN,
            hair: [30, 24, 24],
            hair_style: Hair::Bun,
            clothes: [40, 54, 130],
            top: Top::Scarf([220, 50, 60]),
            ..base
        },
        "Librarian" => Look {
            skin: DEEP,
            hair: [200, 200, 205],
            hair_style: Hair::Curly,
            glasses: Glasses::Round,
            clothes: [130, 50, 72],
            top: Top::Cardigan,
            ..base
        },
        "Museum guide" => Look {
            skin: PALE,
            hair: [110, 70, 40],
            hat: Hat::Peaked([70, 80, 190]),
            glasses: Glasses::Round,
            clothes: [70, 80, 190],
            top: Top::Jacket(true),
            ..base
        },
        "Market trader" => Look {
            skin: OLIVE,
            hair: [40, 30, 30],
            hat: Hat::Flat([130, 96, 60]),
            beard: Beard::Moustache,
            clothes: [190, 60, 50],
            top: Top::Apron([235, 230, 210]),
            ..base
        },
        "Hotel concierge" => Look {
            skin: BROWN,
            hair: [24, 20, 20],
            hat: Hat::Pillbox([190, 30, 40]),
            clothes: [140, 28, 40],
            top: Top::BowTie,
            ..base
        },
        "Ticket seller" => Look {
            skin: PALE,
            hair: [196, 96, 40],
            hair_style: Hair::Long,
            clothes: [250, 130, 30],
            top: Top::HiVis,
            ..base
        },
        "Barista" => Look {
            skin: BROWN,
            hair: [30, 24, 22],
            hat: Hat::Beanie([214, 170, 50]),
            beard: Beard::Full,
            clothes: [40, 40, 46],
            top: Top::Apron([120, 78, 46]),
            ..base
        },
        "Taxi driver" => Look {
            skin: OLIVE,
            hair: [150, 150, 150],
            hat: Hat::Checked,
            glasses: Glasses::Dark,
            beard: Beard::Moustache,
            clothes: [100, 62, 40],
            top: Top::Jacket(false),
            ..base
        },
        _ => base,
    }
}

include!(concat!(env!("OUT_DIR"), "/photos.rs"));

/// The file name a suspect's photo goes under, without .png: "Jon Orlo"
/// becomes "jon-orlo". tools/portrait.sh makes the same names.
pub fn photo_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// One dossier picture per suspect: their photo from assets/suspects/ if
/// there is one, otherwise a portrait drawn from their traits.
pub fn suspect_portraits(suspects: &[Suspect]) -> Vec<Texture2D> {
    suspects
        .iter()
        .map(|s| {
            let name = photo_name(&s.name);
            match PHOTOS.iter().find(|(n, _)| *n == name) {
                Some((_, bytes)) => {
                    let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
                    texture.set_filter(FilterMode::Nearest);
                    texture
                }
                None => portrait(&suspect_look(s)).texture(),
            }
        })
        .collect()
}

/// Work out a suspect's look from the words in their traits: hair colour
/// and style, beards and glasses. Skin comes from their "skin" field if
/// they have one; that and their clothes are otherwise picked from their
/// name, so they look the same every time.
fn suspect_look(s: &Suspect) -> Look {
    let mut rng = Rng::new(hash(&s.name));
    let words_of = |text: &str| -> Vec<String> {
        text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect()
    };
    let all: Vec<String> = s.traits.values().flat_map(|v| words_of(v)).collect();
    let hair: Vec<String> = s.traits.iter().filter(|(k, _)| k.contains("hair")).flat_map(|(_, v)| words_of(v)).collect();
    let any = |list: &[String], options: &[&str]| list.iter().any(|w| options.contains(&w.as_str()));

    let colours: [(&[&str], Rgb); 9] = [
        (&["black"], [30, 24, 24]),
        (&["brown", "brunette"], [100, 64, 40]),
        (&["blond", "blonde", "fair"], [230, 200, 120]),
        (&["red", "ginger", "auburn"], [190, 90, 40]),
        (&["grey", "gray", "white", "silver"], [200, 200, 205]),
        (&["pink"], [240, 130, 180]),
        (&["blue"], [70, 110, 220]),
        (&["green"], [60, 160, 80]),
        (&["purple"], [140, 80, 190]),
    ];
    let natural = [[30, 24, 24], [70, 46, 30], [100, 64, 40], [200, 160, 90]];
    let hair_colour = colours
        .iter()
        .find(|(names, _)| any(&hair, names))
        .map_or(natural[rng.below(natural.len())], |(_, c)| *c);
    let hair_style = if any(&hair, &["bald"]) {
        Hair::Bald
    } else if any(&hair, &["bun"]) {
        Hair::Bun
    } else if any(&hair, &["curly", "wavy", "afro", "curls"]) {
        Hair::Curly
    } else if any(&hair, &["long", "ponytail", "braid", "braids", "plait"]) {
        Hair::Long
    } else if any(&hair, &["short"]) {
        Hair::Short
    } else {
        match s.pronoun.as_str() {
            "she" => Hair::Long,
            "they" => Hair::Curly,
            _ => Hair::Short,
        }
    };
    let beard = if any(&all, &["beard", "bearded"]) {
        Beard::Full
    } else if any(&all, &["moustache", "mustache", "tache"]) {
        Beard::Moustache
    } else {
        Beard::None
    };
    let glasses = if any(&all, &["sunglasses", "shades"]) {
        Glasses::Dark
    } else if any(&all, &["glasses", "specs", "spectacles"]) {
        Glasses::Round
    } else {
        Glasses::None
    };
    let tones = [PALE, ROSY, OLIVE, TAN, BROWN, DEEP];
    let skin = match s.skin.as_deref() {
        Some("light") => PALE,
        Some("rosy") => ROSY,
        Some("olive") => OLIVE,
        Some("tan") => TAN,
        Some("brown") => BROWN,
        Some("dark") => DEEP,
        _ => tones[rng.below(tones.len())],
    };
    let clothes: [Rgb; 8] =
        [[60, 80, 170], [150, 40, 50], [40, 120, 80], [90, 90, 100], [200, 150, 40], [120, 60, 140], [40, 40, 46], [170, 100, 60]];
    let accent = [[220, 50, 60], [240, 200, 60], [60, 160, 220], [240, 240, 236]][rng.below(4)];
    let top = match rng.below(4) {
        0 => Top::ShirtTie(accent),
        1 => Top::Jacket(false),
        2 => Top::Cardigan,
        _ => Top::Scarf(accent),
    };
    let clothes = if let Top::ShirtTie(_) = top { WHITE } else { clothes[rng.below(clothes.len())] };
    Look { skin, hair: hair_colour, hair_style, hat: Hat::None, glasses, beard, clothes, top }
}

/// One portrait per entry in PLACES, in the same order.
pub fn portraits() -> Vec<Texture2D> {
    PLACES.iter().map(|p| portrait(&look(p.witness)).texture()).collect()
}

fn portrait(l: &Look) -> Canvas {
    let mut cv = Canvas::new(PORTRAIT_W, PORTRAIT_H);
    let shade = |c: Rgb| scale(c, 0.72);

    // Long hair hangs behind the head and shoulders.
    if let Hair::Long = l.hair_style {
        cv.ellipse(CX, 40, 19, 24, OUTLINE);
        cv.ellipse(CX, 40, 18, 23, l.hair);
        cv.rect(CX - 18, 40, 36, 32, l.hair);
    }

    // Shoulders: rounded, with an outline and dithered shading on the right.
    for y in 68..PORTRAIT_H {
        let t = (y - 68) as f32 / (PORTRAIT_H - 68) as f32;
        let half = (17.0 + 16.0 * t.sqrt()) as i32;
        cv.rect(CX - half - 1, y, 2 * half + 3, 1, OUTLINE);
        for x in CX - half..=CX + half {
            let right = (x - CX) as f32 / half as f32;
            let c = if right > 0.35 && dither(x, y, (right - 0.35) * 1.5) { shade(l.clothes) } else { l.clothes };
            cv.set(x, y, c);
        }
    }
    cv.rect(CX - 17, 67, 35, 1, OUTLINE);
    draw_top(&mut cv, l);

    // Neck, ears and head.
    cv.rect(CX - 7, 54, 14, 15, OUTLINE);
    cv.rect(CX - 6, 54, 12, 15, shade(l.skin));
    for ex in [CX - 15, CX + 15] {
        cv.ellipse(ex, 41, 3, 5, OUTLINE);
        cv.ellipse(ex, 41, 2, 4, l.skin);
    }
    cv.ellipse(CX, 39, 16, 20, OUTLINE);
    cv.ellipse(CX, 39, 15, 19, l.skin);
    for y in 20..59 {
        for x in CX + 6..CX + 16 {
            if cv.get(x, y) == Some(l.skin) && dither(x, y, (x - CX - 6) as f32 / 12.0) {
                cv.set(x, y, shade(l.skin));
            }
        }
    }

    // Face.
    let brow = if matches!(l.hair_style, Hair::Bald) { shade(l.skin) } else { scale(l.hair, 0.8) };
    for x in [CX - 8, CX + 3] {
        cv.rect(x, 33, 6, 1, brow);
        cv.rect(x, 36, 5, 3, WHITE);
        cv.rect(x + 2, 37, 2, 2, OUTLINE);
    }
    cv.rect(CX, 39, 1, 7, shade(l.skin));
    cv.rect(CX - 2, 46, 4, 1, shade(l.skin));
    cv.rect(CX - 4, 50, 9, 1, [140, 50, 50]);
    cv.rect(CX - 3, 51, 7, 1, scale(l.skin, 0.85));
    if l.skin == PALE || l.skin == ROSY {
        for (x, y) in [(CX - 9, 44), (CX - 8, 45), (CX + 8, 44), (CX + 9, 45)] {
            cv.set(x, y, [236, 150, 140]);
        }
    }

    match l.beard {
        Beard::None => {}
        Beard::Moustache => moustache(&mut cv, l.hair),
        Beard::Full => {
            for y in 44..60 {
                for x in CX - 16..=CX + 16 {
                    let skin = cv.get(x, y).is_some_and(|c| c == l.skin || c == shade(l.skin));
                    let mouth = (49..=51).contains(&y) && (CX - 4..=CX + 4).contains(&x);
                    if skin && !mouth && (y > 46 || dither(x, y, 0.5)) {
                        cv.set(x, y, l.hair);
                    }
                }
            }
            moustache(&mut cv, l.hair);
        }
    }

    // Hair on top, unless a hat covers it.
    let covered = !matches!(l.hat, Hat::None | Hat::Visor);
    if !covered {
        match l.hair_style {
            Hair::Short | Hair::Long | Hair::Bun => {
                cv.ellipse(CX, 25, 16, 8, l.hair);
                cv.rect(CX - 16, 25, 4, 10, l.hair);
                cv.rect(CX + 13, 25, 4, 10, l.hair);
                if let Hair::Bun = l.hair_style {
                    cv.disc(CX, 14, 6, OUTLINE);
                    cv.disc(CX, 14, 5, l.hair);
                }
            }
            Hair::Curly => {
                for (x, y) in [(-12, 26), (-6, 22), (0, 21), (6, 22), (12, 26), (-15, 31), (15, 31)] {
                    cv.disc(CX + x, y, 5, l.hair);
                }
                for (x, y) in [(-8, 20), (2, 19), (10, 23)] {
                    cv.set(CX + x, y, scale(l.hair, 0.7));
                }
            }
            Hair::Bald => {
                cv.rect(CX - 16, 33, 3, 8, l.hair);
                cv.rect(CX + 14, 33, 3, 8, l.hair);
            }
        }
    }

    match l.glasses {
        Glasses::None => {}
        Glasses::Round => {
            for x in [CX - 10, CX + 1] {
                outline_rect(&mut cv, x, 34, 9, 7, OUTLINE);
            }
            cv.rect(CX - 1, 36, 2, 1, OUTLINE);
        }
        Glasses::Dark => {
            for x in [CX - 10, CX + 1] {
                cv.rect(x, 35, 9, 5, [20, 20, 32]);
                cv.set(x + 2, 36, [120, 140, 200]);
            }
            cv.rect(CX - 1, 36, 2, 1, OUTLINE);
        }
    }

    draw_hat(&mut cv, l);
    cv
}

fn moustache(cv: &mut Canvas, hair: Rgb) {
    cv.rect(CX - 6, 47, 13, 2, hair);
    cv.rect(CX - 6, 49, 2, 2, hair);
    cv.rect(CX + 5, 49, 2, 2, hair);
}

fn outline_rect(cv: &mut Canvas, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
    cv.rect(x, y, w, 1, c);
    cv.rect(x, y + h - 1, w, 1, c);
    cv.rect(x, y, 1, h, c);
    cv.rect(x + w - 1, y, 1, h, c);
}

fn draw_top(cv: &mut Canvas, l: &Look) {
    let collar = |cv: &mut Canvas| {
        for i in 0..6 {
            cv.rect(CX - 7 + i, 68 + i, 3, 1, WHITE);
            cv.rect(CX + 5 - i, 68 + i, 3, 1, WHITE);
        }
    };
    match l.top {
        Top::ShirtTie(tie) => {
            collar(cv);
            cv.rect(CX - 2, 69, 5, 4, scale(tie, 0.8));
            for y in 73..PORTRAIT_H {
                let w = if y < 90 { 3 + (y - 73) / 8 } else { 5 };
                cv.rect(CX - w / 2, y, w, 1, tie);
            }
        }
        Top::Jacket(badge) => {
            // An open V showing the shirt, with lapels either side.
            for y in 68..PORTRAIT_H {
                let w = ((y - 68) / 3).min(7);
                cv.rect(CX - w, y, 2 * w + 1, 1, WHITE);
                cv.set(CX - w - 1, y, scale(l.clothes, 0.6));
                cv.set(CX + w + 1, y, scale(l.clothes, 0.6));
            }
            cv.rect(CX, 72, 1, 24, [200, 200, 205]);
            if badge {
                cv.disc(CX + 15, 84, 4, OUTLINE);
                cv.disc(CX + 15, 84, 3, GOLD);
                cv.set(CX + 15, 84, [200, 140, 20]);
            }
        }
        Top::Scarf(scarf) => {
            for i in 0..12 {
                cv.rect(CX - 10 + i, 67 + i, 21 - 2 * i, 1, scarf);
            }
            cv.disc(CX, 69, 3, scale(scarf, 0.75));
            cv.rect(CX + 12, 80, 8, 3, GOLD); // wings badge
        }
        Top::Cardigan => {
            collar(cv);
            cv.rect(CX, 74, 1, 22, scale(l.clothes, 0.6));
            for y in (77..PORTRAIT_H).step_by(6) {
                cv.rect(CX - 2, y, 2, 2, [230, 220, 200]);
            }
        }
        Top::Apron(apron) => {
            cv.rect(CX - 12, 68, 3, 10, apron);
            cv.rect(CX + 10, 68, 3, 10, apron);
            cv.rect(CX - 12, 78, 25, 18, apron);
            cv.rect(CX - 12, 78, 25, 1, scale(apron, 0.8));
            cv.rect(CX - 5, 86, 11, 6, scale(apron, 0.85));
        }
        Top::HiVis => {
            // Reflective bands, only where there's vest underneath.
            let vest = [l.clothes, scale(l.clothes, 0.72)];
            for y in [80, 88] {
                for x in 0..PORTRAIT_W {
                    for (dy, c) in [(0, [240, 240, 120]), (1, [210, 210, 210]), (2, [240, 240, 120])] {
                        if cv.get(x, y + dy).is_some_and(|p| vest.contains(&p)) {
                            cv.set(x, y + dy, c);
                        }
                    }
                }
            }
            cv.rect(CX - 12, 68, 4, 28, [240, 240, 120]);
            cv.rect(CX + 9, 68, 4, 28, [240, 240, 120]);
            collar(cv);
        }
        Top::BowTie => {
            for y in 68..PORTRAIT_H {
                let w = ((y - 68) / 4).min(5);
                cv.rect(CX - w, y, 2 * w + 1, 1, WHITE);
            }
            for i in 0..4 {
                cv.rect(CX - 6 + i, 69 - i / 2, 1, 3 + i, OUTLINE);
                cv.rect(CX + 6 - i, 69 - i / 2, 1, 3 + i, OUTLINE);
            }
            cv.rect(CX - 1, 70, 3, 2, OUTLINE);
            for y in (76..PORTRAIT_H).step_by(6) {
                cv.rect(CX - 9, y, 2, 2, GOLD);
                cv.rect(CX + 8, y, 2, 2, GOLD);
            }
        }
    }
}

fn draw_hat(cv: &mut Canvas, l: &Look) {
    match l.hat {
        Hat::None => {}
        Hat::Visor => {
            cv.rect(CX - 16, 25, 33, 3, [40, 150, 80]);
            cv.rect(CX - 18, 28, 37, 3, [70, 190, 110]);
            cv.rect(CX - 18, 30, 37, 1, [30, 110, 60]);
        }
        Hat::Peaked(c) => {
            cv.ellipse(CX, 19, 18, 8, OUTLINE);
            cv.ellipse(CX, 19, 17, 7, c);
            for y in 14..24 {
                for x in CX + 5..CX + 18 {
                    if cv.get(x, y) == Some(c) && dither(x, y, 0.5) {
                        cv.set(x, y, scale(c, 0.75));
                    }
                }
            }
            cv.rect(CX - 16, 22, 33, 5, scale(c, 0.6));
            cv.rect(CX - 17, 27, 35, 3, OUTLINE);
            cv.rect(CX - 15, 27, 12, 1, [90, 90, 110]);
            cv.disc(CX, 19, 3, GOLD);
            cv.set(CX, 19, [200, 140, 20]);
        }
        Hat::Flat(c) => flat_cap(cv, |x, y| if dither(x, y, 0.4) { scale(c, 0.8) } else { c }),
        Hat::Checked => flat_cap(cv, |x, y| if (x / 2 + y / 2) % 2 == 0 { [235, 235, 230] } else { [30, 30, 34] }),
        Hat::Pillbox(c) => {
            cv.rect(CX - 10, 9, 21, 13, OUTLINE);
            cv.rect(CX - 9, 10, 19, 11, c);
            cv.rect(CX - 9, 10, 19, 2, scale(c, 1.25));
            cv.rect(CX - 9, 18, 19, 2, GOLD);
        }
        Hat::Beanie(c) => {
            cv.ellipse(CX, 22, 17, 12, OUTLINE);
            cv.ellipse(CX, 22, 16, 11, c);
            cv.rect(CX - 17, 22, 35, 10, OUTLINE);
            cv.rect(CX - 16, 23, 33, 8, scale(c, 0.85));
            for x in (CX - 15..CX + 16).step_by(3) {
                cv.rect(x, 23, 1, 8, scale(c, 0.65));
                cv.rect(x, 12, 1, 10, scale(c, 0.9));
            }
            cv.disc(CX, 9, 3, scale(c, 1.1));
        }
    }
}

fn flat_cap(cv: &mut Canvas, cloth: impl Fn(i32, i32) -> Rgb) {
    cv.ellipse(CX, 22, 18, 8, OUTLINE);
    for y in 14..31 {
        for x in CX - 17..=CX + 17 {
            let (dx, dy) = ((x - CX) as f32 / 17.0, (y - 22) as f32 / 7.0);
            if dx * dx + dy * dy <= 1.0 {
                cv.set(x, y, cloth(x, y));
            }
        }
    }
    cv.rect(CX - 19, 28, 39, 3, OUTLINE);
    cv.rect(CX - 18, 28, 37, 2, cloth(0, 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_names_match_the_script() {
        assert_eq!(photo_name("Jon Orlo"), "jon-orlo");
        assert_eq!(photo_name("  Uncle  Erich! "), "uncle-erich");
    }
}
