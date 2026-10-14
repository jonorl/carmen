//! The headquarters printer: text queued with `print` comes out one letter
//! at a time onto continuous paper, which feeds up as lines fill.

use std::collections::VecDeque;

use macroquad::prelude::*;

use crate::device::sound::Sounds;
use crate::ui::{INK, PAPER, Ui, fill, hex, outline};

/// Characters per line; the paper font is monospaced, so wrapping is by count.
const COLS: usize = 23;
const CHAR_W: f32 = 8.0;
const LINE: f32 = 17.0;
/// Characters printed per second.
const SPEED: f64 = 75.0;
/// Queued between paragraphs: printing stops here until `resume`, since
/// only about ten lines fit on the visible paper.
const PAUSE: char = '\u{c}';
/// Queued between paragraphs of a printout nobody needs to page through:
/// printing stops here for `BEAT_SECONDS`, as the original's did.
const BEAT: char = '\u{b}';
const BEAT_SECONDS: f64 = 1.0;
/// Lines of paper showing above the printer.
const SHOWN: usize = 11;

pub struct Printer {
    /// Lines on the paper; the last is the one being printed.
    lines: Vec<String>,
    pending: VecDeque<char>,
    /// The column the queued text will end at, for wrapping what comes next.
    col: usize,
    owed: f64,
    /// How long printing has been resting at a `BEAT`.
    rested: f64,
    /// Pixels of paper still to scroll up after a new line, for a smooth feed.
    feed: f32,
    /// Total paper fed, so the tractor holes move with it.
    fed: f32,
}

impl Printer {
    pub fn new() -> Printer {
        Printer { lines: vec![String::new()], pending: VecDeque::new(), col: 0, owed: 0.0, rested: 0.0, feed: 0.0, fed: 0.0 }
    }

    fn push(&mut self, c: char) {
        self.pending.push_back(c);
        self.col = if c == '\n' { 0 } else { self.col + 1 };
    }

    /// Queue text for printing. '\n' starts a new line; words wrap at the
    /// paper's edge.
    pub fn print(&mut self, text: &str) {
        for (i, para) in text.split('\n').enumerate() {
            if i > 0 {
                self.push('\n');
            }
            for word in para.split_whitespace() {
                let len = word.chars().count();
                if self.col > 0 && self.col + 1 + len > COLS {
                    self.push('\n');
                } else if self.col > 0 {
                    self.push(' ');
                }
                for c in word.chars() {
                    if self.col >= COLS {
                        self.push('\n');
                    }
                    self.push(c);
                }
            }
        }
    }

    /// Queue paragraphs with a blank line and a moment's rest between them,
    /// pausing for the player wherever the next one would push the first
    /// unread line off the top of the paper.
    pub fn print_paragraphs(&mut self, paras: &[String]) {
        let mut unread = 0;
        for (n, para) in paras.iter().enumerate() {
            let rows = rows(para);
            if n > 0 {
                if unread + 1 + rows > SHOWN {
                    self.pause();
                    unread = 0;
                } else {
                    self.pending.push_back(BEAT);
                    unread += 1;
                }
                self.print("\n\n");
            }
            self.print(para);
            unread += rows;
        }
    }

    /// Put text straight onto the current line, for what the player typed.
    pub fn type_now(&mut self, text: &str) {
        self.finish();
        let line = self.lines.last_mut().expect("there is always a current line");
        line.push_str(text);
        self.col += text.chars().count();
    }

    /// Stop after what's queued so far and wait for `resume`.
    pub fn pause(&mut self) {
        self.pending.push_back(PAUSE);
    }

    /// Still printing (not finished, and not waiting at a pause).
    pub fn busy(&self) -> bool {
        self.pending.front().is_some_and(|&c| c != PAUSE)
    }

    /// Stopped at a pause, waiting for the player.
    pub fn waiting(&self) -> bool {
        self.pending.front() == Some(&PAUSE)
    }

    pub fn resume(&mut self) {
        if self.waiting() {
            self.pending.pop_front();
        }
    }

    /// Print the rest of the current paragraph at once (the player skipped),
    /// stopping at the next pause.
    pub fn finish(&mut self) {
        while self.busy() {
            if let Some(c) = self.pending.pop_front() {
                self.put(c);
            }
        }
        self.owed = 0.0;
        self.rested = 0.0;
        self.feed = 0.0;
    }

    fn put(&mut self, c: char) {
        if c == BEAT {
            return;
        }
        if c == '\n' {
            self.lines.push(String::new());
            self.feed += LINE;
            self.fed += LINE;
        } else if let Some(line) = self.lines.last_mut() {
            line.push(c);
        }
    }

    /// Advance printing by `dt` seconds. Returns true on the frame printing
    /// stops, at a pause or the end.
    pub fn update(&mut self, dt: f32, sounds: &mut Option<Sounds>) -> bool {
        self.feed = (self.feed - dt * LINE * 10.0).max(0.0);
        let resting = self.pending.front() == Some(&BEAT);
        if let Some(s) = sounds {
            s.printing(self.busy() && !resting);
        }
        if !self.busy() {
            self.owed = 0.0;
            return false;
        }
        if resting {
            // Nothing is owed for the rest, or it would all print at once afterwards.
            self.owed = 0.0;
            self.rested += dt as f64;
            if self.rested >= BEAT_SECONDS {
                self.pending.pop_front();
                self.rested = 0.0;
            }
            return false;
        }
        self.owed += dt as f64 * SPEED;
        while self.owed >= 1.0 && self.busy() && self.pending.front() != Some(&BEAT) {
            let Some(c) = self.pending.pop_front() else {
                break;
            };
            self.owed -= 1.0;
            self.put(c);
        }
        !self.busy()
    }

    /// Draw the paper and the printer into r (the picture frame's inside).
    /// `echo` is text being typed on the current line, shown with a cursor.
    pub fn draw(&self, ui: &Ui, r: Rect, echo: Option<&str>) {
        let at = |x: f32, y: f32, w: f32, h: f32| Rect::new(r.x + x, r.y + y, w, h);
        let slot = 196.0; // where the paper goes into the printer
        // Only as much paper sticks out as has been printed on.
        let out = (self.lines.len() as f32 * LINE + 14.0 - self.feed).clamp(0.0, slot);
        let top = slot - out;
        let paper = at(26.0, top, 222.0, out);
        fill(paper, PAPER);

        // Tractor-feed holes down both edges, moving up with the paper.
        let phase = 12.0 - (self.fed - self.feed).rem_euclid(12.0);
        let mut y = phase - 12.0;
        while y < slot {
            if y < top {
                y += 12.0;
                continue;
            }
            for x in [30.0, 240.0] {
                let hole = at(x, y.max(0.0), 4.0, (3.0f32).min(3.0 + y));
                if hole.h > 0.0 {
                    fill(hole, hex(0x222222));
                }
            }
            y += 12.0;
        }
        fill(at(38.0, top, 1.0, out), hex(0xbbbbbb));
        fill(at(235.0, top, 1.0, out), hex(0xbbbbbb));

        // Lines, newest at the bottom just above the slot.
        let text_x = r.x + 42.0;
        let base = r.y + slot - 7.0 + self.feed;
        let count = self.lines.len();
        for (i, line) in self.lines.iter().enumerate() {
            let y = base - (count - 1 - i) as f32 * LINE;
            if y - 13.0 < r.y {
                continue;
            }
            let mut text = line.clone();
            if i == count - 1
                && let Some(echo) = echo
            {
                text.push_str(echo);
            }
            ui.mono(&text, text_x, y, INK);
        }

        // The print head sits after the last character on the current line.
        let current = self.lines.last().map_or(0, |l| l.chars().count()) + echo.map_or(0, |e| e.chars().count());
        let head_x = text_x + (current.min(COLS) as f32) * CHAR_W;
        let head = Rect::new(head_x, base - 13.0, 7.0, 16.0);
        fill(head, hex(0xd8d8d8));
        outline(head, INK);
        fill(Rect::new(head.x + 2.0, head.y + 2.0, 1.0, head.h - 4.0), PAPER);

        // The printer: paper guides, a beige case with buttons, and a blue base.
        let beige = hex(0xe3d3b8);
        let shade = hex(0xcbb89a);
        for x in [16.0, 244.0] {
            let guide = at(x, 184.0, 14.0, 20.0);
            fill(guide, beige);
            outline(guide, INK);
            fill(at(x + 2.0, 186.0, 3.0, 14.0), hex(0xf4ecdc));
        }
        let case = at(6.0, 200.0, 262.0, 34.0);
        fill(case, beige);
        fill(at(7.0, 201.0, 260.0, 2.0), hex(0xf4ecdc));
        fill(at(7.0, 222.0, 260.0, 11.0), shade);
        fill(at(7.0, 221.0, 260.0, 1.0), hex(0x8a7a60));
        outline(case, INK);
        fill(at(34.0, 199.0, 206.0, 3.0), INK); // the slot the paper disappears into
        for (n, x) in [196.0, 210.0, 224.0].into_iter().enumerate() {
            let button = at(x, 208.0, 10.0, 4.0);
            fill(button, if n == 0 { hex(0xf0a040) } else { hex(0xd88a30) });
            outline(button, hex(0x6a4a20));
        }
        let base_case = at(4.0, 234.0, 266.0, 34.0);
        fill(base_case, hex(0x6f8db3));
        fill(at(5.0, 235.0, 264.0, 10.0), hex(0x8aa6c8));
        outline(base_case, INK);
        let tray = at(20.0, 254.0, 234.0, 8.0);
        fill(tray, hex(0x2a4470));
        outline(tray, INK);
    }
}

/// How many lines `text` takes up, printed from the start of a line.
fn rows(text: &str) -> usize {
    let mut p = Printer::new();
    p.print(text);
    1 + p.pending.iter().filter(|&&c| c == '\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printed(text: &str) -> Vec<String> {
        let mut p = Printer::new();
        p.print(text);
        p.finish();
        p.lines
    }

    #[test]
    fn wraps_words_at_the_paper_edge() {
        let lines = printed("Detective at keyboard, please identify yourself:");
        assert!(lines.iter().all(|l| l.chars().count() <= COLS));
        assert_eq!(lines[0], "Detective at keyboard,");
        assert_eq!(lines.join(" "), "Detective at keyboard, please identify yourself:");
    }

    #[test]
    fn stops_at_a_pause_until_resumed() {
        let mut p = Printer::new();
        p.print("One\n");
        p.pause();
        p.print("Two");
        p.finish();
        assert!(p.waiting() && !p.busy());
        assert_eq!(p.lines, ["One", ""]);
        p.resume();
        assert!(p.busy());
        p.finish();
        assert_eq!(p.lines, ["One", "Two"]);
        assert!(!p.busy() && !p.waiting());
    }

    #[test]
    fn pauses_before_a_paragraph_that_would_push_unread_lines_off_the_paper() {
        let para = |rows: usize| vec!["word"; rows].join("\n");
        let mut p = Printer::new();
        // Five lines, a blank, then five more fit; the third paragraph waits.
        p.print_paragraphs(&[para(5), para(5), para(3)]);
        p.finish();
        assert!(p.waiting());
        assert_eq!(p.lines.len(), 11);
        p.resume();
        p.finish();
        assert!(!p.waiting() && !p.busy());
        assert_eq!(p.lines.len(), 15);
    }

    #[test]
    fn rests_between_paragraphs_without_printing_anything() {
        let mut p = Printer::new();
        p.print_paragraphs(&["One".to_string(), "Two".to_string()]);
        p.update(1.0, &mut None);
        assert_eq!(p.lines, ["One"]);
        assert!(p.busy(), "resting counts as busy, so a click skips it");
        p.update(0.5, &mut None);
        assert_eq!(p.lines, ["One"]);
        p.update(0.6, &mut None);
        p.update(1.0, &mut None);
        assert_eq!(p.lines, ["One", "", "Two"]);
    }

    #[test]
    fn keeps_blank_lines() {
        assert_eq!(printed("One\n\nTwo"), ["One", "", "Two"]);
    }
}
