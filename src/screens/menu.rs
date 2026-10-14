//! The menu bar across the top, with the language toggle at its right.

use crate::*;

/// The menus across the top of the screen.
pub(crate) fn menus() -> [&'static str; 4] {
    tr(["Game", "Options", "Bureau", "Dossiers"], ["Juego", "Opciones", "Agencia", "Fichas"])
}

/// Where each menu's title sits in the bar.
pub(crate) fn menu_titles(ui: &Ui) -> Vec<Rect> {
    // Closer together on a phone, to leave room for the language toggle.
    let (pad, gap) = if TALL_NOW.get() { (MENU_PAD_TALL, 0.0) } else { (MENU_PAD, 4.0) };
    let mut x = 2.0;
    menus()
        .iter()
        .map(|title| {
            let w = ui.width(title, BIG, true) + 2.0 * pad;
            let r = Rect::new(x, 0.0, w, MENU_H);
            x += w + gap;
            r
        })
        .collect()
}

/// Where the language toggle's two halves sit, at the right of the menu bar.
pub(crate) fn language_buttons(ui: &Ui) -> [(Lang, Rect); 2] {
    let mut x = lay().w - 4.0;
    [Lang::Es, Lang::En].map(|l| {
        let w = ui.width(&l.code().to_uppercase(), SMALL, true) + 8.0;
        x -= w;
        let r = Rect::new(x, 1.0, w, MENU_H - 2.0);
        x -= 2.0;
        (l, r)
    })
}

/// Space either side of a menu's title.
pub(crate) const MENU_PAD: f32 = 7.0;
pub(crate) const MENU_PAD_TALL: f32 = 4.0;

impl App {
    /// The items in menu `m`: a label, and what choosing it does (None when
    /// it can't be chosen right now).
    pub(crate) fn menu_items(&self, m: usize) -> Vec<(String, Option<Action>)> {
        let in_case = matches!(self.screen, Screen::Case(_));
        match m {
            0 => {
                let mut items = vec![
                    (tr("New case", "Nuevo caso").to_string(), in_case.then_some(Action::NewCase)),
                    (tr("How to play", "Cómo jugar").to_string(), Some(Action::Help)),
                    (tr("About", "Acerca de").to_string(), Some(Action::About(true))),
                ];
                if CAN_QUIT {
                    items.push((tr("Quit", "Salir").to_string(), Some(Action::Quit)));
                }
                items
            }
            1 => match &self.sounds {
                Some(s) => vec![(sound_label(s.on).to_string(), Some(Action::ToggleSound))],
                None => vec![(tr("Sound: unavailable", "Sonido: no disponible").to_string(), None)],
            },
            2 => vec![(tr("Detective roster", "Lista de detectives").to_string(), Some(Action::OpenRoster))],
            _ => self.data.suspects.iter().enumerate().map(|(i, s)| (s.name.clone(), Some(Action::OpenDossier(i)))).collect(),
        }
    }

    /// Draw the menu bar and any open menu, and return what was chosen.
    pub(crate) fn menu_bar(&mut self, titles: &[Rect]) -> Option<Action> {
        let ui = &self.ui;
        fill(Rect::new(0.0, 0.0, lay().w, MENU_H), PAPER);
        for (i, (title, r)) in menus().iter().zip(titles).enumerate() {
            let open = self.menu == Some(i);
            if open {
                fill(*r, INK);
            }
            // The first letter in colour, like the original's keyboard shortcuts.
            let first_len = title.chars().next().map_or(0, char::len_utf8);
            let (first, rest) = title.split_at(first_len);
            let pad = if TALL_NOW.get() { MENU_PAD_TALL } else { MENU_PAD };
            let w = ui.text(first, r.x + pad, 13.0, BIG, if open { CYAN } else { RED }, true);
            ui.text(rest, r.x + pad + w - 1.0, 13.0, BIG, if open { PAPER } else { INK }, true);
        }
        // The language toggle at the right-hand end: the one in use in
        // inverse video. Its clicks are taken at the start of the frame.
        for (l, r) in language_buttons(&self.ui) {
            let current = lang() == l;
            if current {
                fill(r, INK);
            } else {
                outline(r, INK);
            }
            self.ui.text_centred(&l.code().to_uppercase(), r.x + r.w / 2.0, 12.0, SMALL, if current { PAPER } else { INK }, true);
        }
        let mut chosen = None;
        let m = self.menu?;
        let items = self.menu_items(m);
        let width = items.iter().map(|(label, _)| self.ui.width(label, BIG, true)).fold(0.0, f32::max) + 24.0;
        let rows = items.len() as f32;
        let x = titles[m].x.min(lay().w - width - 2.0);
        let r = Rect::new(x, MENU_H, width, rows * 19.0 + 6.0);
        fill(Rect::new(r.x + 3.0, r.y + 3.0, r.w, r.h), INK);
        fill(r, PAPER);
        outline(r, INK);
        outline(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0), INK);
        for (k, (label, action)) in items.into_iter().enumerate() {
            let row = Rect::new(r.x + 3.0, r.y + 3.0 + k as f32 * 19.0, r.w - 6.0, 19.0);
            let active = action.is_some();
            let hover = active && self.ui.hovered(row);
            if hover {
                fill(row, INK);
            }
            let colour = if hover {
                PAPER
            } else if active {
                INK
            } else {
                GREY
            };
            self.ui.text(&label, row.x + 8.0, row.y + 15.0, BIG, colour, true);
            if hover && self.ui.take_click(row) {
                chosen = action;
            }
        }
        // Choosing something, clicking elsewhere or pressing Escape closes it.
        let elsewhere = self.ui.take_click(everywhere());
        if chosen.is_some() || elsewhere || is_key_pressed(KeyCode::Escape) {
            self.menu = None;
        }
        chosen
    }
}
