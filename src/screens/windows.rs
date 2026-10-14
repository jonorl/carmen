//! The windows that open over any screen: a suspect's dossier, the detective
//! roster and About.

use crate::*;

/// What a dossier needs: the suspects and their photos.
pub(crate) struct Files<'a> {
    pub(crate) data: &'a Data,
    pub(crate) mugshots: &'a [Texture2D],
}

/// A suspect's file, as in the original: a white sheet over the screen with
/// their name on a tab, their photo in a heavy frame, bold labels with
/// typewritten answers, and their background along the bottom. The arrow
/// keys leaf through the files; a click or Escape closes it.
pub(crate) fn dossier_window(ui: &mut Ui, files: &Files, i: usize) -> Option<Action> {
    let s = &files.data.suspects[i];
    let count = files.data.suspects.len();
    let r = lay().dossier;

    fill(Rect::new(r.x + 5.0, r.y + 5.0, r.w, r.h), INK);
    fill(r, PAPER);
    outline(r, INK);
    outline(Rect::new(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0), INK);
    let name = s.name.to_uppercase();
    let tab_w = (name.chars().count() as f32 * MONO_W + 40.0).max(160.0);
    let tab = Rect::new(r.x + r.w - tab_w - 14.0, r.y - 20.0, tab_w, 24.0);
    fill(tab, PAPER);
    outline(tab, INK);
    outline(Rect::new(tab.x + 3.0, tab.y + 3.0, tab.w - 6.0, tab.h), INK);
    // The tab opens into the sheet, so clear the sheet's border under it.
    fill(Rect::new(tab.x + 4.0, r.y - 1.0, tab.w - 8.0, 5.0), PAPER);
    ui.mono(&name, tab.x + (tab.w - name.chars().count() as f32 * MONO_W) / 2.0, tab.y + 18.0, INK);

    // Side by side on a wide screen; the photo above the facts on a tall one.
    let tall = TALL_NOW.get();
    let photo =
        if tall { Rect::new(r.x + (r.w - 144.0) / 2.0, r.y + 14.0, 144.0, 204.0) } else { Rect::new(r.x + 14.0, r.y + 14.0, 144.0, 204.0) };
    fill(photo, INK);
    outline(Rect::new(photo.x + 3.0, photo.y + 3.0, photo.w - 6.0, photo.h - 6.0), PAPER);
    let params = DrawTextureParams { dest_size: Some(vec2(132.0, 192.0)), ..Default::default() };
    draw_texture_ex(&files.mugshots[i], photo.x + 6.0, photo.y + 6.0, WHITE, params);

    let mut facts: Vec<(&str, &str)> = vec![(tr("Name", "Nombre"), &s.name), (tr("Sex", "Sexo"), data::sex_shown(&s.sex))];
    for t in &files.data.traits {
        facts.push((t.label(), s.shown(t)));
    }
    let right = r.x + r.w - 14.0;
    let (x, mut y) = if tall { (r.x + 14.0, photo.y + photo.h + 24.0) } else { (photo.x + photo.w + 14.0, r.y + 30.0) };
    for (label, value) in facts {
        y = labelled(ui, label, value, (x, right), y);
    }
    if let Some(bio) = &s.bio {
        let y = if tall { y + 2.0 } else { (y + 2.0).max(photo.y + photo.h + 20.0) };
        labelled(ui, tr("Notes", "Notas"), bio.get(), (r.x + 14.0, right), y);
    }
    // Phones have no arrow keys; the Dossiers menu still opens any file.
    let n = i + 1;
    let hint = match (tall, lang()) {
        (true, Lang::En) => format!("File {n} of {count}. Tap to close."),
        (false, Lang::En) => format!("File {n} of {count}. Left and right arrows: other files. Click: close."),
        (true, Lang::Es) => format!("Ficha {n} de {count}. Tocá para cerrar."),
        (false, Lang::Es) => format!("Ficha {n} de {count}. Flechas: otras fichas. Clic: cerrar."),
    };
    ui.text_centred(&hint, r.x + r.w / 2.0, r.y + r.h - 8.0, SMALL, DARK, false);

    if ui.key(KeyCode::Left) {
        return Some(Action::OpenDossier((i + count - 1) % count));
    }
    if ui.key(KeyCode::Right) {
        return Some(Action::OpenDossier((i + 1) % count));
    }
    let close = ui.take_click(Rect::new(0.0, MENU_H, lay().w, lay().h)) || ui.key(KeyCode::Escape) || ui.key(KeyCode::Enter);
    close.then_some(Action::CloseDossier)
}

/// Width of a character in the monospaced font.
pub(crate) const MONO_W: f32 = 8.0;

/// The bureau's detectives, from the server, or from this device when the
/// server can't be reached.
pub(crate) struct Roster {
    pub(crate) request: Option<net::Request>,
    pub(crate) list: Option<Vec<carmen_shared::Detective>>,
    /// Whether the list is only this device's.
    pub(crate) offline: bool,
    /// The first row shown.
    pub(crate) top: usize,
}

pub(crate) const ROSTER_ROW_H: f32 = 20.0;

impl Roster {
    pub(crate) fn open() -> Roster {
        Roster { request: progress::roster_request(), list: None, offline: false, top: 0 }
    }

    /// Take in the server's answer once it's there, and scroll to `me`.
    pub(crate) fn poll(&mut self, progress: &Progress, me: &str) {
        if self.list.is_some() {
            return;
        }
        let answer = match &mut self.request {
            Some(request) => match request.poll() {
                None => return,
                Some(answer) => answer.ok().and_then(|body| serde_json::from_str::<Vec<carmen_shared::Detective>>(&body).ok()),
            },
            None => None,
        };
        self.offline = answer.is_none();
        let list = answer.unwrap_or_else(|| progress.local_roster());
        // Start with me on screen, a few rows down if there's room.
        let me = carmen_shared::key(me);
        if let Some(at) = list.iter().position(|d| carmen_shared::key(&d.name) == me) {
            self.top = at.saturating_sub(3);
        }
        self.list = Some(list);
        self.request = None;
    }
}

/// The detective roster, after the original's: a white window with a title
/// bar, everyone's name, rank and arrests in a list that scrolls, the
/// detective at the keyboard in inverse video, and a button to close it.
pub(crate) fn roster_window(ui: &mut Ui, roster: &mut Roster, me: &str) -> Option<Action> {
    let r = lay().dossier;
    fill(Rect::new(r.x + 5.0, r.y + 5.0, r.w, r.h), INK);
    fill(r, PAPER);
    outline(r, INK);
    outline(Rect::new(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0), INK);

    // The title between two blue bars.
    let title = tr("Detective Roster", "Lista de detectives");
    let tw = ui.width(title, BIG, true);
    let bar = Rect::new(r.x + 8.0, r.y + 9.0, r.w - 16.0, 16.0);
    let gap = Rect::new(bar.x + (bar.w - tw) / 2.0 - 8.0, bar.y, tw + 16.0, bar.h);
    fill(Rect::new(bar.x, bar.y, gap.x - bar.x, bar.h), NAVY);
    fill(Rect::new(gap.x + gap.w, bar.y, bar.x + bar.w - gap.x - gap.w, bar.h), NAVY);
    ui.text(title, gap.x + 8.0, bar.y + 13.0, BIG, INK, true);

    let list_box = Rect::new(r.x + 10.0, r.y + 50.0, r.w - 20.0, r.h - 90.0);
    let bar_w = 18.0;
    let rows_box = Rect::new(list_box.x, list_box.y, list_box.w - bar_w, list_box.h);
    // Columns: the name from the left, rank and arrests centred.
    let name_x = rows_box.x + 12.0;
    let rank_cx = rows_box.x + rows_box.w * if TALL_NOW.get() { 0.6 } else { 0.62 };
    let arrests_cx = rows_box.x + rows_box.w * if TALL_NOW.get() { 0.9 } else { 0.88 };
    ui.text(tr("Name", "Nombre"), name_x, list_box.y - 7.0, BIG, INK, true);
    ui.text_centred(tr("Rank", "Rango"), rank_cx, list_box.y - 7.0, BIG, INK, true);
    ui.text_centred(tr("Arrests", "Arrestos"), arrests_cx, list_box.y - 7.0, BIG, INK, true);
    outline(list_box, INK);

    let shown = ((rows_box.h - 4.0) / ROSTER_ROW_H).floor().max(1.0) as usize;
    let total = roster.list.as_ref().map_or(0, Vec::len);
    let most = total.saturating_sub(shown);
    match &roster.list {
        None => {
            let checking = tr("Checking the bureau's records…", "Revisando los archivos de la agencia…");
            ui.text_centred(checking, rows_box.x + rows_box.w / 2.0, rows_box.y + 30.0, SMALL, DARK, false)
        }
        Some(list) if list.is_empty() => {
            let none = tr("No detectives on file yet.", "Todavía no hay detectives.");
            ui.text_centred(none, rows_box.x + rows_box.w / 2.0, rows_box.y + 30.0, SMALL, DARK, false)
        }
        Some(list) => {
            let me = carmen_shared::key(me);
            for (n, d) in list.iter().skip(roster.top).take(shown).enumerate() {
                let row = Rect::new(rows_box.x + 2.0, rows_box.y + 2.0 + n as f32 * ROSTER_ROW_H, rows_box.w - 3.0, ROSTER_ROW_H);
                let mine = carmen_shared::key(&d.name) == me;
                if mine {
                    fill(row, INK);
                }
                let ink = if mine { PAPER } else { INK };
                let base = row.y + 15.0;
                // Names this device filed in lower case read better capitalised.
                let mut name: String = d.name.clone();
                if let Some(first) = name.chars().next().filter(|c| c.is_lowercase()) {
                    name.replace_range(..first.len_utf8(), &first.to_uppercase().to_string());
                }
                ui.mono(&name, name_x, base, ink);
                let rank = RANKS[rank_for(d.arrests)].label();
                ui.mono(rank, rank_cx - rank.chars().count() as f32 * MONO_W / 2.0, base, ink);
                let n = d.arrests.to_string();
                ui.mono(&n, arrests_cx - n.chars().count() as f32 * MONO_W / 2.0, base, ink);
            }
        }
    }

    // The scroll bar: arrows at each end, a grey track and a thumb.
    let track = Rect::new(rows_box.x + rows_box.w, list_box.y, bar_w, list_box.h);
    outline(track, INK);
    let up = Rect::new(track.x, track.y, bar_w, bar_w);
    let down = Rect::new(track.x, track.y + track.h - bar_w, bar_w, bar_w);
    let groove = Rect::new(track.x + 1.0, up.y + up.h, bar_w - 2.0, down.y - up.y - up.h);
    fill(groove, GREY);
    if total > shown {
        let thumb_h = (groove.h * shown as f32 / total as f32).max(10.0);
        let thumb_y = groove.y + (groove.h - thumb_h) * roster.top as f32 / most.max(1) as f32;
        let thumb = Rect::new(groove.x, thumb_y, groove.w, thumb_h);
        fill(thumb, PAPER);
        outline(thumb, INK);
    }
    for (b, pointing_up) in [(up, true), (down, false)] {
        fill(b, PAPER);
        outline(b, INK);
        let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        let (tip, base) = if pointing_up { (cy - 4.0, cy + 3.0) } else { (cy + 4.0, cy - 3.0) };
        draw_triangle(vec2(cx, tip), vec2(cx - 5.0, base), vec2(cx + 5.0, base), INK);
    }
    let mut step = 0i32;
    if ui.take_click(up) || ui.key(KeyCode::Up) {
        step = -1;
    } else if ui.take_click(down) || ui.key(KeyCode::Down) {
        step = 1;
    } else if ui.key(KeyCode::PageUp) {
        step = -(shown as i32);
    } else if ui.key(KeyCode::PageDown) {
        step = shown as i32;
    } else if ui.hovered(list_box) {
        step = -ui.wheel as i32;
    }
    roster.top = (roster.top as i32 + step).clamp(0, most as i32) as usize;

    if roster.offline {
        let note =
            tr("Bureau unreachable: this device's records only.", "Sin conexión con la agencia: solo los datos de este dispositivo.");
        ui.text(&ui.fit(note, SMALL, false, r.w - 150.0), r.x + 12.0, r.y + r.h - 16.0, SMALL, DARK, false);
    }
    let close_r = Rect::new(r.x + r.w - 120.0, r.y + r.h - 32.0, 108.0, 22.0);
    let outside = !ui.hovered(r) && ui.take_click(Rect::new(0.0, MENU_H, lay().w, lay().h));
    let close = close_button(ui, close_r) || outside || ui.key(KeyCode::Escape) || ui.key(KeyCode::Enter);
    close.then_some(Action::CloseRoster)
}

/// A yellow button reading "Close" with its key, Esc, in inverse, like the
/// original's buttons. Whether it was clicked.
pub(crate) fn close_button(ui: &mut Ui, close_r: Rect) -> bool {
    let hover = ui.hovered(close_r);
    fill(close_r, if hover { INK } else { YELLOW });
    outline(close_r, INK);
    // "Close" and its key in inverse, like the original's buttons.
    let (ink, paper) = if hover { (YELLOW, INK) } else { (INK, YELLOW) };
    let close = tr("Close", "Cerrar");
    let (cw, kw) = (ui.width(close, BIG, true), ui.width("Esc", SMALL, true) + 6.0);
    let x = close_r.x + (close_r.w - cw - 6.0 - kw) / 2.0;
    ui.text(close, x, close_r.y + 16.0, BIG, ink, true);
    let key = Rect::new(x + cw + 6.0, close_r.y + 5.0, kw, 13.0);
    fill(key, ink);
    ui.text("Esc", key.x + 3.0, key.y + 10.0, SMALL, paper, true);
    ui.take_click(close_r)
}

pub(crate) const ABOUT: &str = "This is a recreation of the 1990s Where in the World is Carmen Sandiego? from Brøderbund Software, \
                     remade in Rust and compiled to WebAssembly. The suspects are friends and family, and this was a project \
                     with the sole intention of having fun.";
pub(crate) const ABOUT_ES: &str = "Esta es una recreación del clásico de los 90 ¿Dónde en el mundo está Carmen Sandiego?, de Brøderbund \
                        Software, rehecho en Rust y compilado a WebAssembly. Los sospechosos son amigos y familia, y este \
                        proyecto se hizo con el único fin de divertirse.";
/// (label in English, in Spanish, as shown, address).
pub(crate) const LINKS: [(&str, &str, &str, &str); 2] = [
    ("Code", "Código", "github.com/jonorl/carmen", "https://github.com/jonorl/carmen"),
    ("Website", "Sitio web", "jonathan-orlowski.dev", "https://jonathan-orlowski.dev"),
];

/// About the game, laid out like the original's: a white box with the title,
/// a red rule, then what it is and where to find more.
pub(crate) fn about_window(ui: &mut Ui) -> Option<Action> {
    let w = if TALL_NOW.get() { lay().w - 16.0 } else { 440.0 };
    let cols = ((w - 28.0) / MONO_W) as usize;
    let rows = ui::wrap_cols(tr(ABOUT, ABOUT_ES), cols);
    let h = 44.0 + rows.len() as f32 * 17.0 + 12.0 + LINKS.len() as f32 * 19.0 + 44.0;
    let r = Rect::new(((lay().w - w) / 2.0).round(), ((lay().h - h) / 2.0).round(), w, h);
    fill(Rect::new(r.x + 5.0, r.y + 5.0, r.w, r.h), INK);
    fill(r, PAPER);
    outline(r, INK);
    outline(Rect::new(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0), INK);

    let x = r.x + 14.0;
    ui.text(game_title(), x, r.y + 24.0, BIG, INK, true);
    fill(Rect::new(x, r.y + 32.0, r.w - 28.0, 2.0), RED);
    let mut y = r.y + 52.0;
    for row in &rows {
        ui.mono(row, x, y, INK);
        y += 17.0;
    }
    y += 12.0;
    let mut action = None;
    for (en, es, shown, url) in LINKS {
        let lw = ui.text(&format!("{}:", tr(en, es)), x, y, BIG, INK, true) + 8.0;
        let link = Rect::new(x + lw - 2.0, y - 14.0, shown.chars().count() as f32 * MONO_W + 4.0, 18.0);
        let hover = ui.hovered(link);
        if hover {
            fill(link, NAVY);
        }
        ui.mono(shown, x + lw, y, if hover { PAPER } else { NAVY });
        fill(Rect::new(x + lw, y + 2.0, link.w - 4.0, 1.0), if hover { PAPER } else { NAVY });
        if ui.take_click(link) {
            action = Some(Action::OpenLink(url));
        }
        y += 19.0;
    }
    let close_r = Rect::new(r.x + r.w - 120.0, r.y + r.h - 32.0, 108.0, 22.0);
    let outside = !ui.hovered(r) && ui.take_click(Rect::new(0.0, MENU_H, lay().w, lay().h));
    if close_button(ui, close_r) || outside || ui.key(KeyCode::Escape) || ui.key(KeyCode::Enter) {
        action = Some(Action::About(false));
    }
    action
}

/// "Label: answer" with the label in bold and the answer typewritten,
/// wrapping under the label. Returns the baseline for the next one.
pub(crate) fn labelled(ui: &Ui, label: &str, value: &str, (x, right): (f32, f32), y: f32) -> f32 {
    let lw = ui.text(&format!("{label}:"), x, y, BIG, INK, true) + 8.0;
    let first = ((right - x - lw) / MONO_W) as usize;
    let rest = ((right - x) / MONO_W) as usize;
    let mut rows: Vec<String> = vec![String::new()];
    for word in value.split_whitespace() {
        let limit = if rows.len() == 1 { first } else { rest };
        let row = rows.last_mut().expect("starts with one row");
        let used = row.chars().count();
        if used > 0 && used + 1 + word.chars().count() > limit {
            rows.push(word.to_string());
        } else {
            if used > 0 {
                row.push(' ');
            }
            row.push_str(word);
        }
    }
    let mut y = y;
    for (n, row) in rows.iter().enumerate() {
        ui.mono(row, if n == 0 { x + lw } else { x }, y, INK);
        y += 17.0;
    }
    y + 4.0
}
