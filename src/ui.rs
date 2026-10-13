//! Drawing helpers for the retro look: the palette, pixel text, double-line
//! frames, buttons, icons and the mouse pointer. Everything is drawn at
//! 640x360 and scaled up with hard pixel edges, like a DOS game.

use macroquad::prelude::*;

pub const fn hex(v: u32) -> Color {
    Color::new(((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0)
}

// The 16-colour EGA palette the original games were drawn in, more or less.
pub const INK: Color = hex(0x000000);
pub const PAPER: Color = hex(0xffffff);
pub const GREY: Color = hex(0xaaaaaa);
pub const DARK: Color = hex(0x555555);
pub const NAVY: Color = hex(0x0000aa);
pub const BLUE: Color = hex(0x5555ff);
pub const GREEN: Color = hex(0x00aa00);
pub const LIME: Color = hex(0x55ff55);
pub const CYAN: Color = hex(0x55ffff);
pub const RED: Color = hex(0xff5555);
pub const YELLOW: Color = hex(0xffff55);
pub const TILE: Color = hex(0xe8e8e8);

pub const BIG: u16 = 16;
pub const SMALL: u16 = 13;

pub fn line_height(size: u16) -> f32 {
    if size >= BIG { 19.0 } else { 15.0 }
}

/// Characters Pixelify Sans doesn't have; they're drawn with Noto Sans instead.
const MISSING: [char; 1] = ['₹'];

/// Split text into runs, each drawn with the pixel font or the fallback.
fn runs(s: &str) -> Vec<(&str, bool)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if MISSING.contains(&c) {
            if start < i {
                out.push((&s[start..i], false));
            }
            out.push((&s[i..i + c.len_utf8()], true));
            start = i + c.len_utf8();
        }
    }
    if start < s.len() {
        out.push((&s[start..], false));
    }
    out
}

pub struct Ui {
    pixel: Font,
    fallback: Font,
    /// VT323, a monospaced terminal font, for the printer paper.
    mono: Font,
    /// Mouse position in layout pixels.
    pub mouse: Vec2,
    /// How wide this frame's layout is, and where it sits in the window.
    width: f32,
    offset: Vec2,
    scale: f32,
    clicked: bool,
    /// 1 for a notch up, -1 for a notch down, 0 when the wheel is still.
    pub wheel: f32,
    /// False while drawing the screen underneath a pop-up window, so it
    /// ignores the mouse and keyboard.
    pub enabled: bool,
}

impl Ui {
    pub fn new(mut pixel: Font, fallback: Font, mut mono: Font) -> Ui {
        pixel.set_filter(FilterMode::Nearest);
        mono.set_filter(FilterMode::Nearest);
        Ui {
            pixel,
            fallback,
            mono,
            mouse: Vec2::ZERO,
            width: 640.0,
            offset: Vec2::ZERO,
            scale: 1.0,
            clicked: false,
            wheel: 0.0,
            enabled: true,
        }
    }

    /// Printer text: VT323 at 20 px, where its pixels line up and every
    /// character is 8 px wide.
    pub fn mono(&self, s: &str, x: f32, y: f32, colour: Color) {
        let params = TextParams { font: Some(&self.mono), font_size: 20, color: colour, ..Default::default() };
        draw_text_ex(s, x.round(), y.round(), params);
    }

    /// Start a frame drawn `width` wide, at `scale` and `offset` in the window.
    pub fn begin_frame(&mut self, offset: Vec2, scale: f32, width: f32) {
        (self.width, self.offset, self.scale) = (width, offset, scale);
        self.mouse = (Vec2::from(mouse_position()) - offset) / scale;
        self.clicked = is_mouse_button_pressed(MouseButton::Left);
        self.enabled = true;
        // Not signum(): that returns 1.0 for 0.0, so a still wheel would scroll.
        let y = mouse_wheel().1;
        self.wheel = if y > 0.0 {
            1.0
        } else if y < 0.0 {
            -1.0
        } else {
            0.0
        };
    }

    /// Where layout rectangle `r` is in the window.
    pub fn to_window(&self, r: Rect) -> Rect {
        Rect::new(self.offset.x + r.x * self.scale, self.offset.y + r.y * self.scale, r.w * self.scale, r.h * self.scale)
    }

    fn font(&self, fallback: bool) -> &Font {
        if fallback { &self.fallback } else { &self.pixel }
    }

    pub fn width(&self, s: &str, size: u16, bold: bool) -> f32 {
        let w: f32 = runs(s).iter().map(|(r, fb)| measure_text(r, Some(self.font(*fb)), size, 1.0).width).sum();
        w + if bold { 1.0 } else { 0.0 }
    }

    /// Draw text with its baseline at y; returns the width drawn. Bold is
    /// faked by drawing twice, a pixel apart, which gives the chunky DOS look.
    pub fn text(&self, s: &str, x: f32, y: f32, size: u16, colour: Color, bold: bool) -> f32 {
        let (mut x, y) = (x.round(), y.round());
        let start = x;
        for (run, fb) in runs(s) {
            let params = TextParams { font: Some(self.font(fb)), font_size: size, color: colour, ..Default::default() };
            if bold {
                draw_text_ex(run, x + 1.0, y, params.clone());
            }
            x += draw_text_ex(run, x, y, params).width;
        }
        x - start + if bold { 1.0 } else { 0.0 }
    }

    pub fn text_centred(&self, s: &str, cx: f32, y: f32, size: u16, colour: Color, bold: bool) {
        self.text(s, cx - self.width(s, size, bold) / 2.0, y, size, colour, bold);
    }

    pub fn wrap(&self, s: &str, size: u16, bold: bool, max_w: f32) -> Vec<String> {
        let mut rows = Vec::new();
        let mut row = String::new();
        for word in s.split_whitespace() {
            let candidate = if row.is_empty() { word.to_string() } else { format!("{row} {word}") };
            if !row.is_empty() && self.width(&candidate, size, bold) > max_w {
                rows.push(std::mem::replace(&mut row, word.to_string()));
            } else {
                row = candidate;
            }
        }
        rows.push(row);
        rows
    }

    /// Shorten text with an ellipsis so it fits in max_w.
    pub fn fit(&self, s: &str, size: u16, bold: bool, max_w: f32) -> String {
        if self.width(s, size, bold) <= max_w {
            return s.to_string();
        }
        let mut cut = s.to_string();
        while !cut.is_empty() && self.width(&format!("{cut}…"), size, bold) > max_w {
            cut.pop();
        }
        format!("{}…", cut.trim_end())
    }

    pub fn hovered(&self, r: Rect) -> bool {
        self.enabled && r.contains(self.mouse)
    }

    pub fn key(&self, k: KeyCode) -> bool {
        self.enabled && is_key_pressed(k)
    }

    pub fn any_key(&self) -> bool {
        self.enabled && get_last_key_pressed().is_some()
    }

    /// True once per click, so one click can't press two things.
    pub fn take_click(&mut self, r: Rect) -> bool {
        if self.clicked && self.hovered(r) {
            self.clicked = false;
            true
        } else {
            false
        }
    }

    /// A menu line in a text panel. Hovering shows it in inverse video, the
    /// way DOS menus highlighted a choice.
    pub fn option(&mut self, r: Rect, label: &str, detail: &str, active: bool) -> bool {
        let hover = active && self.hovered(r);
        if hover {
            fill(r, PAPER);
        }
        let colour = match (active, hover) {
            (false, _) => DARK,
            (true, true) => INK,
            (true, false) => PAPER,
        };
        let base = r.y + r.h - 5.0;
        let detail_w = if detail.is_empty() { 0.0 } else { self.width(detail, SMALL, false) + 8.0 };
        let label = self.fit(label, BIG, true, r.w - detail_w - 10.0);
        self.text(&label, r.x + 4.0, base, BIG, colour, true);
        if !detail.is_empty() {
            let c = if hover { DARK } else { GREY };
            self.text(detail, r.x + r.w - detail_w + 4.0, base - 1.0, SMALL, c, false);
        }
        hover && self.take_click(r)
    }

    /// One of the four big buttons under the text panel.
    pub fn icon_button(&mut self, r: Rect, icon: Icon, selected: bool, active: bool) -> bool {
        let hover = active && self.hovered(r);
        frame(r, if selected { GREEN } else { PAPER }, if selected { 4.0 } else { 3.0 });
        let inner = inner(r);
        fill(inner, if active { TILE } else { GREY });
        draw_icon(self, inner, icon, active);
        if hover {
            outline(Rect::new(inner.x, inner.y, inner.w, inner.h), YELLOW);
            // A tooltip above the buttons, since the icons carry no words for some.
            let w = self.width(icon.name(), SMALL, true) + 10.0;
            let x = (r.x + r.w / 2.0 - w / 2.0).clamp(2.0, self.width - w - 2.0);
            fill(Rect::new(x, r.y - 18.0, w, 16.0), INK);
            outline(Rect::new(x, r.y - 18.0, w, 16.0), PAPER);
            self.text(icon.name(), x + 5.0, r.y - 6.0, SMALL, PAPER, true);
        }
        hover && self.take_click(r)
    }
}

/// Word-wrap for the monospaced font, where every character is the same width.
pub fn wrap_cols(text: &str, cols: usize) -> Vec<String> {
    let mut rows = vec![String::new()];
    for word in text.split_whitespace() {
        let row = rows.last_mut().expect("starts with one row");
        let used = row.chars().count();
        if used > 0 && used + 1 + word.chars().count() > cols {
            rows.push(word.to_string());
        } else {
            if used > 0 {
                row.push(' ');
            }
            row.push_str(word);
        }
    }
    rows
}

pub fn fill(r: Rect, c: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, c);
}

/// A filled rectangle with its corners clipped, like a rounded box at low resolution.
pub fn rounded(r: Rect, c: Color) {
    fill(Rect::new(r.x + 3.0, r.y, r.w - 6.0, r.h), c);
    fill(Rect::new(r.x, r.y + 3.0, r.w, r.h - 6.0), c);
    fill(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0), c);
}

/// A 1-pixel border inside r.
pub fn outline(r: Rect, c: Color) {
    fill(Rect::new(r.x, r.y, r.w, 1.0), c);
    fill(Rect::new(r.x, r.y + r.h - 1.0, r.w, 1.0), c);
    fill(Rect::new(r.x, r.y, 1.0, r.h), c);
    fill(Rect::new(r.x + r.w - 1.0, r.y, 1.0, r.h), c);
}

/// The original's double-line border: a thick outer band, a black gap and a thin inner line.
pub fn frame(r: Rect, edge: Color, thickness: f32) {
    fill(r, edge);
    let t = thickness;
    fill(Rect::new(r.x + t, r.y + t, r.w - 2.0 * t, r.h - 2.0 * t), INK);
    outline(Rect::new(r.x + t + 2.0, r.y + t + 2.0, r.w - 2.0 * t - 4.0, r.h - 2.0 * t - 4.0), PAPER);
}

/// The drawable area inside a frame.
pub fn inner(r: Rect) -> Rect {
    Rect::new(r.x + 7.0, r.y + 7.0, r.w - 14.0, r.h - 14.0)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Notes,
    Depart,
    Investigate,
    Crime,
}

impl Icon {
    pub fn name(self) -> &'static str {
        match self {
            Icon::Notes => "Case notes",
            Icon::Depart => "Depart by plane",
            Icon::Investigate => "Investigate",
            Icon::Crime => "Crime computer",
        }
    }
}

fn draw_icon(ui: &Ui, r: Rect, icon: Icon, active: bool) {
    let ink = if active { INK } else { DARK };
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    match icon {
        Icon::Notes => {
            ui.text_centred("NOTES", cx, r.y + 12.0, SMALL, ink, true);
            let page = Rect::new(cx - 13.0, r.y + 16.0, 26.0, 24.0);
            fill(page, PAPER);
            outline(page, ink);
            for i in 0..4 {
                fill(Rect::new(page.x + 4.0, page.y + 6.0 + i as f32 * 4.0, 18.0, 1.0), if active { BLUE } else { GREY });
            }
            for i in 0..5 {
                fill(Rect::new(page.x + 3.0 + i as f32 * 5.0, page.y - 2.0, 2.0, 4.0), ink);
            }
        }
        Icon::Depart => {
            ui.text_centred("DEPART", cx, r.y + 12.0, SMALL, ink, true);
            // A plane seen head-on: wings, engines, fuselage and tail fin.
            let y = cy + 8.0;
            fill(Rect::new(cx - 26.0, y, 52.0, 2.0), ink);
            fill(Rect::new(cx - 1.0, y - 11.0, 2.0, 9.0), ink);
            fill(Rect::new(cx - 7.0, y - 7.0, 14.0, 1.0), ink);
            draw_circle(cx, y + 1.0, 4.0, DARK);
            draw_circle(cx - 12.0, y + 4.0, 2.5, DARK);
            draw_circle(cx + 12.0, y + 4.0, 2.5, DARK);
        }
        Icon::Investigate => {
            // A magnifying glass with a question mark in the lens.
            let (lx, ly) = (cx + 3.0, cy - 3.0);
            draw_line(lx - 8.0, ly + 8.0, lx - 22.0, ly + 19.0, 5.0, ink);
            draw_circle(lx, ly, 13.0, ink);
            draw_circle(lx, ly, 10.0, if active { hex(0xcfe3ff) } else { TILE });
            ui.text_centred("?", lx, ly + 6.0, BIG, if active { NAVY } else { DARK }, true);
        }
        Icon::Crime => {
            // A computer whose screen says CRIME.
            let screen = Rect::new(cx - 26.0, r.y + 4.0, 52.0, 26.0);
            fill(screen, DARK);
            fill(Rect::new(screen.x + 3.0, screen.y + 3.0, screen.w - 6.0, screen.h - 6.0), if active { BLUE } else { GREY });
            ui.text_centred("CRIME", cx, screen.y + 18.0, SMALL, PAPER, true);
            fill(Rect::new(cx - 30.0, r.y + 33.0, 60.0, 7.0), ink);
            for i in 0..9 {
                fill(Rect::new(cx - 27.0 + i as f32 * 6.0, r.y + 35.0, 4.0, 1.0), GREY);
            }
        }
    }
}

/// The DOS-style arrow pointer: X is the outline, . the fill.
const POINTER: [&str; 17] = [
    "X",
    "XX",
    "X.X",
    "X..X",
    "X...X",
    "X....X",
    "X.....X",
    "X......X",
    "X.......X",
    "X........X",
    "X.....XXXXX",
    "X..X..X",
    "X.X X..X",
    "XX  X..X",
    "X    X..X",
    "     X..X",
    "      XX",
];

pub fn draw_pointer(at: Vec2) {
    for (y, row) in POINTER.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let colour = match c {
                'X' => INK,
                '.' => PAPER,
                _ => continue,
            };
            draw_rectangle(at.x.floor() + x as f32, at.y.floor() + y as f32, 1.0, 1.0, colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_by_character_count() {
        assert_eq!(wrap_cols("He ordered ceviche with a glass", 12), ["He ordered", "ceviche with", "a glass"]);
    }

    #[test]
    fn splits_out_missing_glyphs() {
        assert_eq!(runs("the ₹ sign"), [("the ", false), ("₹", true), (" sign", false)]);
        assert_eq!(runs("plain"), [("plain", false)]);
    }
}
