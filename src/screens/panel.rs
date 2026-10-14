//! The text panel on the right of a case: stories, the chase and the walk to
//! jail, henchmen and warnings, interviews, the notes, the crime computer, and
//! the scrolling help pages.

use crate::*;

/// Pictures drawn once at start-up and shared with the case screen.
#[derive(Clone, Copy)]
pub(crate) struct Art<'a> {
    pub(crate) places: &'a [Texture2D],
    pub(crate) portraits: &'a [Texture2D],
    pub(crate) jail: &'a Jail,
    pub(crate) cast: &'a Cast,
}

pub(crate) fn text_panel(ui: &mut Ui, case: &mut Case, r: Rect, art: Art, sounds: &mut Option<Sounds>) -> Option<Action> {
    let portraits = art.portraits;
    let row = |n: f32| Rect::new(r.x, r.y + n, r.w, 20.0);
    let typed = ((get_time() - case.typed_from) * TYPING_SPEED) as usize;
    let skip = ui.any_key();
    let won = case.game.won;
    let scene = match &mut case.panel {
        Panel::March { started, cued, .. } => Some((*started, cued, if won { CAUGHT_WALK } else { ESCAPED_WALK })),
        Panel::Jailed { started, cued, .. } => Some((*started, cued, if won { CAUGHT_JAIL } else { NO_CUES })),
        Panel::Henchman { started, cued, kind, .. } => Some((*started, cued, HENCHMAN_CUES[*kind])),
        Panel::Danger { started, cued, kind, .. } => Some((*started, cued, DANGER_CUES[*kind])),
        _ => None,
    };
    if let Some((started, cued, cues)) = scene {
        let t = get_time() - started;
        while let Some(&(at, cue)) = cues.get(*cued)
            && t >= at
        {
            *cued += 1;
            if let Some(s) = sounds {
                match cue {
                    Cue::Footsteps => s.footsteps(),
                    Cue::Lament => s.lament(),
                    Cue::Triumph => s.triumph(),
                    Cue::Clank => s.clank(),
                    Cue::Quiet => s.quiet(),
                    Cue::Tune(kind) => s.henchman(kind),
                    Cue::Danger(kind) => s.danger(kind),
                }
            }
        }
    }
    // The henchman has gone (and his line has been read), or the warning's
    // tune has finished: on to the interview.
    let unveiled = match &case.panel {
        Panel::Henchman { started, kind, tell, .. } => get_time() - started >= henchman_times(art.cast, *kind, *tell, r).1,
        Panel::Danger { started, kind, .. } => get_time() - started >= DANGER_WITNESS_AT[*kind],
        _ => false,
    };
    if unveiled && let Panel::Henchman { then, .. } | Panel::Danger { then, .. } = std::mem::replace(&mut case.panel, Panel::Notes) {
        case.show(*then);
    }
    if let Panel::Jailed { started, ending, .. } = &mut case.panel
        && get_time() - *started >= print_at(won)
        && let Some(paras) = ending.take()
    {
        case.printer = Printer::new();
        case.printer.print_paragraphs(&paras);
    }
    match &case.panel {
        Panel::Story(lines) => {
            let ended = case.game.over;
            let room = if ended { r.h - 24.0 } else { r.h };
            let done = paragraphs(ui, Rect::new(r.x, r.y, r.w, room), lines, typed);
            if !done && (skip || ui.take_click(r)) {
                case.typed_from = f64::NEG_INFINITY;
            }
            if ended && done {
                if ui.option(Rect::new(r.x, r.y + r.h - 20.0, 120.0, 20.0), tr("New case", "Nuevo caso"), "", true) {
                    return Some(Action::ToTitle);
                }
                if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 70.0, r.y + r.h - 20.0, 70.0, 20.0), tr("Quit", "Salir"), "", true) {
                    return Some(Action::Quit);
                }
            }
            None
        }
        Panel::Chase { started, .. } => {
            chase(ui, art.cast, r, get_time() - started, case.game.thief_is_female());
            let over = get_time() - started >= CHASE_SECONDS;
            (over || skip || ui.take_click(r)).then_some(Action::EndChase)
        }
        Panel::March { started, .. } => {
            let t = get_time() - started;
            let jailed_at = if case.game.won { JAILED_AT } else { EMPTY_JAIL_AT };
            walk_to_jail(art.cast, r, t, case.game.won);
            (t >= jailed_at || skip || ui.take_click(r)).then_some(Action::Jail)
        }
        Panel::Jailed { started, ending, .. } => {
            let t = get_time() - started;
            art.jail.draw(r, won.then_some(t >= CLANK_AT));
            if ending.is_some() {
                // The music is playing out: a click or key skips to the clank.
                if skip || ui.take_click(Rect::new(0.0, MENU_H, lay().w, lay().h - MENU_H)) {
                    if let Some(s) = sounds {
                        s.quiet();
                    }
                    if let Panel::Jailed { started, .. } = &mut case.panel {
                        *started = started.min(get_time() - CLANK_AT);
                    }
                }
                return None;
            }
            let (busy, waiting) = (case.printer.busy(), case.printer.waiting());
            if busy || waiting {
                if waiting {
                    let more = tr("Press a key or click for more.", "Apretá una tecla o hacé clic para seguir.");
                    ui.text(more, r.x, r.y + r.h - 6.0, SMALL, GREY, false);
                }
                // The paper is in the picture frame, so a click anywhere carries on.
                if skip || ui.take_click(Rect::new(0.0, MENU_H, lay().w, lay().h - MENU_H)) {
                    if busy {
                        case.printer.finish();
                    } else {
                        case.printer.resume();
                    }
                }
                return None;
            }
            if ui.option(Rect::new(r.x, r.y + r.h - 20.0, 120.0, 20.0), tr("New case", "Nuevo caso"), "", true) {
                return Some(Action::ToTitle);
            }
            if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 70.0, r.y + r.h - 20.0, 70.0, 20.0), tr("Quit", "Salir"), "", true) {
                return Some(Action::Quit);
            }
            None
        }
        Panel::Henchman { started, kind, tell, .. } => {
            let (t, kind, tell) = (get_time() - started, *kind, *tell);
            let (gone, _) = henchman_times(art.cast, kind, tell, r);
            let appears = t - HENCHMAN_APPEARS[kind];
            match kind {
                0 => art.cast.stride.draw(r, appears),
                1 => art.cast.sneak.draw(r, appears),
                _ => art.cast.popup.draw(r, appears),
            }
            if tell && t >= gone {
                // Low in the panel, like "There goes the suspect!".
                let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h * 0.73 - 15.0);
                let lines = tr(
                    ["A S.U.R.M.E. henchman!", "You must be on", "the right track."],
                    ["¡Un secuaz de S.U.R.M.E.!", "Vas por", "buen camino."],
                );
                for (n, line) in lines.into_iter().enumerate() {
                    ui.text_centred(line, cx, cy + n as f32 * 19.0, BIG, PAPER, true);
                }
            }
            // A click or key skips to his line, or past it to the interview.
            if skip || ui.take_click(r) {
                if let Some(s) = sounds {
                    s.quiet();
                }
                let to = if tell && t < gone { gone } else { f64::INFINITY };
                if let Panel::Henchman { started, cued, .. } = &mut case.panel {
                    *started = get_time() - to.min(1e6);
                    // Skipped before his tune began: it shouldn't start now.
                    *cued = HENCHMAN_CUES[kind].len();
                }
            }
            None
        }
        Panel::Danger { started, kind, .. } => {
            let (t, kind) = (get_time() - started, *kind);
            art.cast.danger.draw(kind, r, t - DANGER_AT[kind]);
            // A click or key goes straight to the witness.
            if skip || ui.take_click(r) {
                if let Some(s) = sounds {
                    s.quiet();
                }
                if let Panel::Danger { started, cued, .. } = &mut case.panel {
                    *started = get_time() - DANGER_WITNESS_AT[kind];
                    *cued = DANGER_CUES[kind].len();
                }
            }
            None
        }
        Panel::Interview { place, said, notes } => {
            let done = interview(ui, r, &portraits[*place], PLACES[*place].witness_label(), said, notes, typed);
            if !done && (skip || ui.take_click(r)) {
                case.typed_from = f64::NEG_INFINITY;
            }
            None
        }
        Panel::Depart => {
            let city = case.game.country(case.game.here).city();
            ui.text(&tr(format!("Flights from {city}:"), format!("Vuelos desde {city}:")), r.x, r.y + 14.0, BIG, YELLOW, true);
            case.hover_dest = None;
            for (n, &d) in case.dests.iter().enumerate() {
                let line = row(28.0 + n as f32 * 24.0);
                if ui.hovered(line) {
                    case.hover_dest = Some(d);
                }
                let c = case.game.country(d);
                let hours = format!("{} h", flight_hours(case.game.country(case.game.here), c));
                if ui.option(line, &c.label(), &hours, true) {
                    return Some(Action::Fly(d));
                }
            }
            ui.text(tr("Or click a red dot on the map.", "O hacé clic en un punto rojo del mapa."), r.x, r.y + 144.0, SMALL, GREY, false);
            None
        }
        Panel::Flying => {
            if let Some(f) = &case.flight {
                let c = case.game.country(f.to);
                let to = c.city();
                ui.text(&tr(format!("Flying to {to}…"), format!("Volando a {to}…")), r.x, r.y + 14.0, BIG, PAPER, true);
                let hours = flight_hours(case.game.country(f.from), c);
                ui.text(&tr(format!("{hours} hours in the air."), format!("{hours} horas de vuelo.")), r.x, r.y + 34.0, SMALL, GREY, false);
            }
            None
        }
        Panel::Notes => notes_panel(ui, case, r, sounds),
        Panel::Crime { editing } => {
            let editing = *editing;
            crime_computer(ui, case, r, editing)
        }
        Panel::Help => {
            ui.text(tr("HOW TO PLAY", "CÓMO JUGAR"), r.x, r.y + 14.0, BIG, YELLOW, true);
            page(ui, Rect::new(r.x, r.y + 22.0, r.w, r.h - 46.0), &help_paras(), &mut case.page_scroll);
            let back = ui.option(row(r.h - 20.0), tr("Back to the notes", "Volver a las notas"), "", true);
            (back || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Notes))
        }
        Panel::GiveUp => {
            ui.text(tr("Give up this case?", "¿Abandonar este caso?"), r.x, r.y + 14.0, BIG, YELLOW, true);
            if ui.option(row(28.0), tr("Yes, give up", "Sí, abandonar"), "", true) {
                return Some(Action::ConfirmGiveUp);
            }
            let no = ui.option(row(52.0), tr("No, keep going", "No, seguir"), "", true);
            (no || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Notes))
        }
    }
}

/// How long the chase lasts: the length of its soundtrack.
pub(crate) const CHASE_SECONDS: f64 = 17.6;

/// The chase, timed to its soundtrack: the suspect runs across the bottom
/// of the panel from `SUSPECT_AT`, "There goes the suspect!", then the police
/// run after them from `POLICE_AT`.
pub(crate) const SUSPECT_AT: f64 = 1.6;
pub(crate) const POLICE_AT: f64 = 14.5;

pub(crate) fn chase(ui: &Ui, cast: &Cast, r: Rect, t: f64, female: bool) {
    cast.suspect.draw(r, t - SUSPECT_AT);
    // Low in the panel, as in the original, which wiped the lines one at a time.
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h * 0.73 - 5.0);
    let (first, second) = match (lang(), female) {
        (Lang::En, _) => ("There goes", "the suspect!"),
        (Lang::Es, false) => ("¡Ahí va", "el sospechoso!"),
        (Lang::Es, true) => ("¡Ahí va", "la sospechosa!"),
    };
    if (5.7..9.8).contains(&t) {
        ui.text_centred(first, cx, cy, BIG, PAPER, true);
    }
    if (5.7..10.6).contains(&t) {
        ui.text_centred(second, cx, cy + 19.0, BIG, PAPER, true);
    }
    cast.police.draw(r, t - POLICE_AT);
}

/// The walk to jail, timed from the original: the suspect and the officer
/// set off across the panel at `MARCH_AT` to their footsteps, and after a
/// moment of empty panel the jail appears at `JAILED_AT`.
pub(crate) const MARCH_AT: f64 = 0.2;
pub(crate) const JAILED_AT: f64 = 9.8;
/// The same when the suspect got away: a sad tune over the empty panel,
/// then the officer trudging across alone from `TRUDGE_AT`.
pub(crate) const TRUDGE_AT: f64 = 7.1;
pub(crate) const EMPTY_JAIL_AT: f64 = 15.1;

/// In jail with the suspect, as in the original: the music plays out, the
/// prisoner grabs the bars with a clank, and then the printer comes up and
/// starts on the ending. An empty cell gets the printer straight away.
pub(crate) const CLANK_AT: f64 = 14.215;
pub(crate) const PRINT_AT: f64 = 14.75;

pub(crate) fn print_at(caught: bool) -> f64 {
    if caught { PRINT_AT } else { 0.0 }
}

/// A henchman's appearance, timed from recordings of the original: his tune
/// starts `HENCHMAN_TUNE_AT` after the walk to the witness, and he shows
/// himself a moment into it (`HENCHMAN_APPEARS`, for each of the three).
pub(crate) const HENCHMAN_TUNE_AT: f64 = 0.9;
pub(crate) const HENCHMAN_APPEARS: [f64; 3] = [HENCHMAN_TUNE_AT + 1.15, HENCHMAN_TUNE_AT + 1.14, HENCHMAN_TUNE_AT + 0.93];
pub(crate) const HENCHMAN_CUES: [&[(f64, Cue)]; 3] =
    [&[(HENCHMAN_TUNE_AT, Cue::Tune(0))], &[(HENCHMAN_TUNE_AT, Cue::Tune(1))], &[(HENCHMAN_TUNE_AT, Cue::Tune(2))]];
/// The final city's warnings, timed from recordings of the original after the
/// walk to the witness: when the weapon shows (`DANGER_AT`), when its sound
/// and tune start, and when the witness takes over as the tune ends.
pub(crate) const DANGER_AT: [f64; 3] = [1.05, 1.12, 0.88];
pub(crate) const DANGER_CUES: [&[(f64, Cue)]; 3] = [&[(1.0, Cue::Danger(0))], &[(1.2, Cue::Danger(1))], &[(1.69, Cue::Danger(2))]];
pub(crate) const DANGER_WITNESS_AT: [f64; 3] = [5.75, 5.05, 5.95];
/// How long "A V.I.L.E. henchman! You must be on the right track." stays up.
pub(crate) const HENCHMAN_LINE_SECONDS: f64 = 3.5;

/// When henchman `kind` has gone from panel `r`, and when the interview
/// takes over (after his line, if he has one).
pub(crate) fn henchman_times(cast: &Cast, kind: usize, tell: bool, r: Rect) -> (f64, f64) {
    let on_screen = match kind {
        0 => cast.stride.seconds(r),
        1 => cast.sneak.seconds(r),
        _ => sprites::PopUp::SECONDS,
    };
    // The original's line came a tenth of a second after he'd gone.
    let gone = HENCHMAN_APPEARS[kind] + on_screen + 0.1;
    (gone, if tell { gone + HENCHMAN_LINE_SECONDS } else { gone })
}

#[derive(Clone, Copy)]
pub(crate) enum Cue {
    Footsteps,
    Lament,
    Triumph,
    Clank,
    Quiet,
    Tune(usize),
    Danger(usize),
}

/// The sounds of the walk to jail and of the jail, and when they start, as
/// in recordings of the original. Walking, only the footsteps are heard
/// (the officer's first step breaking off the sad tune); the music for a
/// capture comes with the jail.
pub(crate) const CAUGHT_WALK: &[(f64, Cue)] = &[(0.0, Cue::Footsteps), (9.5, Cue::Quiet)];
pub(crate) const ESCAPED_WALK: &[(f64, Cue)] = &[(1.0, Cue::Lament), (7.1, Cue::Footsteps), (14.6, Cue::Quiet)];
pub(crate) const CAUGHT_JAIL: &[(f64, Cue)] = &[(0.0, Cue::Triumph), (CLANK_AT, Cue::Clank)];
pub(crate) const NO_CUES: &[(f64, Cue)] = &[];

/// The arrested suspect and the officer, or the officer alone if the
/// suspect wasn't `caught`, walking in from the right and off to the left
/// along the bottom of the panel.
pub(crate) fn walk_to_jail(cast: &Cast, r: Rect, t: f64, caught: bool) {
    if caught {
        cast.march.draw(r, t - MARCH_AT);
    } else {
        cast.slumped.draw(r, t - TRUDGE_AT);
    }
}

/// A line of the case as the printer puts it: the paper's font has straight
/// quotes only, and the summary's columns go one under another because the
/// paper is narrow.
pub(crate) fn printable(line: &Line) -> String {
    let text = line.text.replace(['\u{201c}', '\u{201d}'], "\"").replace("   ", "\n");
    match &line.speaker {
        Some(who) => format!("{who}: {text}"),
        None => text,
    }
}

/// A witness portrait bottom left with their job underneath, and what they
/// said typing out in a speech bubble pointing at them. Returns true once
/// the whole statement is showing.
pub(crate) fn interview(ui: &Ui, r: Rect, portrait: &Texture2D, witness: &str, said: &str, notes: &[Line], typed: usize) -> bool {
    let mut y = r.y;
    for line in notes {
        for row in ui.wrap(&line.text, SMALL, false, r.w) {
            y += line_height(SMALL);
            ui.text(&row, r.x, y - 3.0, SMALL, GREY, false);
        }
    }

    let label_y = r.y + r.h - 3.0;
    let (pw, ph) = (people::PORTRAIT_W as f32, people::PORTRAIT_H as f32);
    let pic = Rect::new(r.x, label_y - 16.0 - ph, pw, ph);
    draw_texture(portrait, pic.x, pic.y, WHITE);
    ui.mono(&witness.to_uppercase(), r.x, label_y, PAPER);

    // The bubble sits beside the portrait and grows upwards with longer statements.
    const CHAR_W: f32 = 8.0;
    const ROW_H: f32 = 17.0;
    let bx = pic.x + pw + 14.0;
    let bw = r.x + r.w - bx;
    let rows = wrap_cols(said, ((bw - 16.0) / CHAR_W) as usize);
    let bottom = pic.y + ph - 4.0;
    let top = (bottom - rows.len() as f32 * ROW_H - 12.0).max(y + 6.0);
    rounded(Rect::new(bx, top, bw, bottom - top), PAPER);
    let tail = (pic.y + 50.0).clamp(top + 8.0, bottom - 8.0);
    draw_triangle(vec2(bx + 1.0, tail - 5.0), vec2(bx + 1.0, tail + 5.0), vec2(bx - 9.0, tail), PAPER);

    let mut left = typed;
    let mut base = top + 19.0;
    for row in &rows {
        let shown: String = row.chars().take(left).collect();
        ui.mono(&shown, bx + 8.0, base, INK);
        left = left.saturating_sub(row.chars().count() + 1);
        base += ROW_H;
    }
    typed >= rows.iter().map(|r| r.chars().count() + 1).sum::<usize>()
}

/// The height `lines` need when wrapped at `size`.
pub(crate) fn paragraphs_height(ui: &Ui, w: f32, lines: &[Line], size: u16, bold: bool) -> f32 {
    let lh = line_height(size);
    lines.iter().map(|l| if l.text.is_empty() { 6.0 } else { ui.wrap(&speech(l), size, bold, w).len() as f32 * lh + 3.0 }).sum()
}

pub(crate) fn speech(l: &Line) -> String {
    match &l.speaker {
        Some(s) => format!("{s}: {}", l.text),
        None => l.text.clone(),
    }
}

/// Draw log lines as wrapped paragraphs, showing only the first `budget`
/// characters: the typewriter effect. Uses the big font when it fits.
/// Returns true once everything is showing.
pub(crate) fn paragraphs(ui: &Ui, r: Rect, lines: &[Line], budget: usize) -> bool {
    let (size, bold) = if paragraphs_height(ui, r.w, lines, BIG, true) <= r.h { (BIG, true) } else { (SMALL, false) };
    let lh = line_height(size);
    let mut y = r.y + lh - 5.0;
    let mut left = budget;
    let mut complete = true;
    for line in lines {
        if line.text.is_empty() {
            y += 6.0;
            continue;
        }
        let (colour, strong) = tone_style(line.tone);
        let bold = bold || strong;
        let speaker = line.speaker.as_ref().map_or(0, |s| s.len() + 1);
        for (n, row) in ui.wrap(&speech(line), size, bold, r.w).iter().enumerate() {
            let count = row.chars().count();
            if left < count {
                complete = false;
            }
            let shown: String = row.chars().take(left).collect();
            left = left.saturating_sub(count + 1);
            let mut x = r.x;
            let sp = if n == 0 { speaker.min(shown.len()) } else { 0 };
            if sp > 0 {
                x += ui.text(&shown[..sp], x, y, size, CYAN, bold);
            }
            ui.text(&shown[sp..], x, y, size, colour, bold);
            y += lh;
        }
        y += 3.0;
    }
    complete
}

pub(crate) fn notes_panel(ui: &mut Ui, case: &mut Case, r: Rect, sounds: &mut Option<Sounds>) -> Option<Action> {
    let game = &case.game;
    ui.text(tr("CASE NOTES", "NOTAS DEL CASO"), r.x, r.y + 14.0, BIG, YELLOW, true);
    let left = (game.deadline - game.t).max(0);
    let due = clock_text(game.deadline);
    let deadline = tr(format!("Deadline {due} ({left} hours left)"), format!("Vence el {due} ({left} h)"));
    ui.text(&deadline, r.x, r.y + 30.0, SMALL, if left <= 12 { RED } else { PAPER }, false);
    let w = ui.text(tr("Warrant: ", "Orden: "), r.x, r.y + 44.0, SMALL, GREY, false);
    match game.warrant_for {
        Some(s) => ui.text(&game.data.suspects[s].name, r.x + w, r.y + 44.0, SMALL, LIME, true),
        None => ui.text(tr("none", "ninguna"), r.x + w, r.y + 44.0, SMALL, GREY, false),
    };

    // The log, newest at the bottom; the mouse wheel scrolls back.
    let log = Rect::new(r.x, r.y + 50.0, r.w, r.h - 76.0);
    fill(Rect::new(r.x, log.y, r.w, 1.0), DARK);
    if ui.hovered(log) && ui.wheel != 0.0 {
        let max = case.rows.len().saturating_sub(4);
        case.notes_scroll = if ui.wheel > 0.0 { (case.notes_scroll + 2).min(max) } else { case.notes_scroll.saturating_sub(2) };
    }
    let end = case.rows.len() - case.notes_scroll.min(case.rows.len());
    let mut y = log.y + log.h - 4.0;
    for row in case.rows[..end].iter().rev() {
        if row.text.is_empty() {
            y -= 5.0;
            continue;
        }
        if y - 11.0 < log.y {
            break;
        }
        let (colour, bold) = tone_style(row.tone);
        let mut x = r.x;
        if row.speaker > 0 {
            x += ui.text(&row.text[..row.speaker], x, y, SMALL, CYAN, bold);
        }
        ui.text(&row.text[row.speaker..], x, y, SMALL, colour, bold);
        y -= line_height(SMALL);
    }
    if case.notes_scroll > 0 {
        let hint = tr("scroll down for newer", "bajá para ver lo nuevo");
        ui.text(hint, r.x + r.w - ui.width(hint, SMALL, false), r.y + 14.0, SMALL, GREY, false);
    }
    fill(Rect::new(r.x, log.y + log.h, r.w, 1.0), DARK);

    let footer = r.y + r.h - 20.0;
    if ui.option(Rect::new(r.x, footer, 124.0, 20.0), tr("How to play", "Cómo jugar"), "", true) {
        return Some(Action::Show(Panel::Help));
    }
    sound_toggle(ui, Rect::new(r.x + 130.0, footer, 112.0, 20.0), sounds);
    if ui.option(Rect::new(r.x + r.w - 82.0, footer, 82.0, 20.0), tr("Give up", "Rendirse"), "", true) {
        return Some(Action::Show(Panel::GiveUp));
    }
    None
}

/// The crime computer: a menu on the monitor's screen in yellow capitals,
/// the row under the mouse in inverse video. Choosing a field lists its
/// values on the screen; results come out on the printer to the left.
pub(crate) fn crime_computer(ui: &mut Ui, case: &mut Case, r: Rect, editing: Option<usize>) -> Option<Action> {
    let computing = case.computing.map(|started| get_time() - started);
    let screen = computer::draw(r, computing);
    // While it's working the menu shows but can't be used.
    let enabled = ui.enabled;
    ui.enabled = enabled && computing.is_none();
    let action = crime_menu(ui, case, screen, editing);
    ui.enabled = enabled;
    action
}

pub(crate) fn crime_menu(ui: &mut Ui, case: &mut Case, screen: Rect, editing: Option<usize>) -> Option<Action> {
    let s = Rect::new(screen.x + 6.0, screen.y + 5.0, screen.w - 12.0, screen.h - 10.0);
    let fields = case.game.warrant_fields();
    let value_of = |case: &Case, key: &str| case.game.warrant.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.clone());

    let Some(f) = editing else {
        // The menu: each field and its current value, then the commands.
        let rows = fields.len() + 3;
        let row_h = (s.h / (rows as f32 + 0.25)).min(17.0).floor();
        for (i, (key, label, values)) in fields.iter().enumerate() {
            let row = Rect::new(s.x, s.y + i as f32 * row_h, s.w, row_h);
            let value = value_of(case, key).unwrap_or_default();
            let shown = if value.is_empty() { String::new() } else { case.game.shown(key, &value) };
            if screen_row(ui, row, &format!("{}:", label.to_uppercase()), &shown, false) {
                return Some(Action::Show(Panel::Crime { editing: Some(i) }));
            }
            // The mouse wheel steps through the values without opening the list.
            if ui.hovered(row) && ui.wheel != 0.0 {
                let options: Vec<Option<&String>> = std::iter::once(None).chain(values.iter().map(Some)).collect();
                let at = options.iter().position(|o| o.is_some_and(|v| game::same(v, &value))).unwrap_or(0);
                let next = (at as i32 - ui.wheel as i32).rem_euclid(options.len() as i32) as usize;
                if let Some(slot) = case.game.warrant.iter_mut().find(|(k, _)| k == key) {
                    slot.1 = options[next].cloned();
                }
            }
        }
        let y = s.y + (fields.len() as f32 + 0.25) * row_h;
        let row = |n: f32| Rect::new(s.x, y + n * row_h, s.w, row_h);
        if screen_row(ui, row(0.0), &tr(format!("COMPUTE ({WARRANT_HOURS} H)"), format!("COMPUTAR ({WARRANT_HOURS} H)")), "", false) {
            return Some(Action::Compute);
        }
        if screen_row(ui, row(1.0), tr("CLEAR", "BORRAR"), "", false) {
            for entry in case.game.warrant.iter_mut() {
                entry.1 = None;
            }
        }
        if screen_row(ui, row(2.0), tr("DOSSIERS", "FICHAS"), "", false) {
            return Some(Action::OpenDossier(0));
        }
        return None;
    };

    // One field's values, with "don't know" first and a way back.
    let (key, label, values) = &fields[f];
    let current = value_of(case, key);
    let rows = values.len() + 3;
    let row_h = (s.h / rows as f32).min(17.0).floor();
    let row = |n: usize| Rect::new(s.x, s.y + n as f32 * row_h, s.w, row_h);
    let size = if row_h >= 16.0 { BIG } else { SMALL };
    ui.text(&format!("{}:", label.to_uppercase()), s.x + 2.0, row(0).y + row_h - 3.0, size, YELLOW, true);
    let mut choices: Vec<(String, Option<String>)> = vec![(tr("DON'T KNOW", "NO SÉ").to_string(), None)];
    choices.extend(values.iter().map(|v| (case.game.shown(key, v), Some(v.clone()))));
    for (i, (text, value)) in choices.iter().enumerate() {
        let chosen = match (value, &current) {
            (Some(v), Some(c)) => game::same(v, c),
            (None, None) => true,
            _ => false,
        };
        if screen_row(ui, row(i + 1), text, "", chosen) {
            if let Some(slot) = case.game.warrant.iter_mut().find(|(k, _)| k == key) {
                slot.1 = value.clone();
            }
            return Some(Action::Show(Panel::Crime { editing: None }));
        }
    }
    let back = screen_row(ui, row(choices.len() + 1), tr("BACK", "VOLVER"), "", false);
    (back || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Crime { editing: None }))
}

/// A line on the computer screen: yellow, or white with blue text under the
/// mouse. `value` follows the label in white; `chosen` marks the current choice.
pub(crate) fn screen_row(ui: &mut Ui, r: Rect, label: &str, value: &str, chosen: bool) -> bool {
    let hover = ui.hovered(r);
    if hover {
        fill(r, PAPER);
    }
    let size = if r.h >= 16.0 { BIG } else { SMALL };
    let base = r.y + r.h - 3.0;
    let x = r.x + 12.0;
    if chosen {
        ui.text(">", r.x + 2.0, base, size, if hover { NAVY } else { LIME }, true);
    }
    let w = ui.text(label, x, base, size, if hover { NAVY } else { YELLOW }, true);
    if !value.is_empty() {
        let fitted = ui.fit(value, size, true, r.w - (x - r.x) - w - 10.0);
        ui.text(&fitted, x + w + 6.0, base, size, if hover { NAVY } else { PAPER }, true);
    }
    hover && ui.take_click(r)
}

pub(crate) struct Para {
    pub(crate) text: String,
    pub(crate) size: u16,
    pub(crate) bold: bool,
    pub(crate) colour: Color,
    pub(crate) gap: f32,
}

pub(crate) fn para(text: impl Into<String>, size: u16, bold: bool, colour: Color, gap: f32) -> Para {
    Para { text: text.into(), size, bold, colour, gap }
}

/// Wrapped paragraphs that scroll with the mouse wheel.
pub(crate) fn page(ui: &Ui, r: Rect, paras: &[Para], scroll: &mut f32) {
    let mut rows = Vec::new();
    for p in paras {
        for row in ui.wrap(&p.text, p.size, p.bold, r.w - 4.0) {
            rows.push((row, p));
        }
        rows.push((String::new(), p));
    }
    let content: f32 = rows.iter().map(|(t, p)| if t.is_empty() { p.gap } else { line_height(p.size) }).sum();
    let max = (content - r.h).max(0.0);
    if ui.hovered(r) {
        *scroll = (*scroll - ui.wheel * 30.0).clamp(0.0, max);
    }
    let mut y = r.y - *scroll;
    for (text, p) in &rows {
        if text.is_empty() {
            y += p.gap;
            continue;
        }
        y += line_height(p.size);
        if y - line_height(p.size) + 3.0 >= r.y && y <= r.y + r.h {
            ui.text(text, r.x, y - 3.0, p.size, p.colour, p.bold);
        }
    }
    if max > 0.0 {
        // A scroll bar, so it's clear there's more.
        let bar_h = (r.h * r.h / content).max(12.0);
        let bar_y = r.y + (r.h - bar_h) * (*scroll / max);
        fill(Rect::new(r.x + r.w - 2.0, r.y, 2.0, r.h), DARK);
        fill(Rect::new(r.x + r.w - 2.0, bar_y, 2.0, bar_h), PAPER);
    }
}

pub(crate) fn help_paras() -> Vec<Para> {
    let body = |t: String| para(t, SMALL, false, PAPER, 6.0);
    let head = |t: &str| para(t, BIG, true, YELLOW, 0.0);
    if lang() == Lang::Es {
        return vec![
            body("Alguien de la familia robó un tesoro y anda saltando por el mundo. Seguí el rastro hasta el escondite y hacé el arresto antes de que venza el plazo.".to_string()),
            body("Cada arresto te acerca a un ascenso, de Aprendiz a Supersabueso, y los casos de cada rango son más difíciles. Tus arrestos se guardan con tu nombre.".to_string()),
            head("Investigar"),
            body(format!("La lupa abre una ventana con los tres lugares de la ciudad, cada uno con un testigo; los que ya visitaste se ven en gris. Si vas por buen camino, los testigos dan pistas sobre el próximo país (banderas, monedas, monumentos, comidas, idiomas...) y a veces describen a quien robó. En una ciudad equivocada nadie vio nada, así que volvé. Cada entrevista lleva {INTERVIEW_HOURS} horas.")),
            head("Viajar"),
            body("El avión. De cada ciudad salen cuatro vuelos; elegí uno de la lista o hacé clic en su punto del mapa. Los vuelos más largos llevan más tiempo.".to_string()),
            head("Computadora"),
            body(format!("Cargá lo que sabés de quien robó y computá ({WARRANT_HOURS} horas). Cuando queda una sola persona, tenés la orden de arresto. Las fichas te ayudan a relacionar pistas con personas.")),
            head("El escondite"),
            body("Cuando los testigos dicen que sigue en la ciudad, estás cerca. Seguí investigando para hacer el arresto, pero necesitás una orden de arresto contra la persona correcta o se te escapa.".to_string()),
            head("Notas y tiempo"),
            body("La libreta guarda todo lo que te dijeron los testigos. El reloj arranca el lunes a las 9 de la mañana y dormís de 11 de la noche a 7 de la mañana. Hacé clic o apretá una tecla para saltear lo que se está escribiendo.".to_string()),
        ];
    }
    vec![
        body("A family member has stolen a treasure and is hopping around the world. Follow the trail to their hideout and arrest them before the deadline.".to_string()),
        body("Arrests get you promoted, from Rookie up to Super Sleuth, and each rank's cases are harder. Your arrests are saved under your name.".to_string()),
        head("Investigate"),
        body(format!("The magnifying glass opens a window showing the city's three places, each with a witness; ones you've visited are grey. On the right trail, witnesses hint at the next country (flags, money, landmarks, food, language...) and sometimes describe the thief. In a wrong city nobody has seen a thing, so fly back. Each interview takes {INTERVIEW_HOURS} hours.")),
        head("Depart"),
        body("The plane. Four flights leave every city; pick one from the list or click its dot on the map. Longer flights take longer.".to_string()),
        head("Crime computer"),
        body(format!("Enter what you've learned about the thief and compute ({WARRANT_HOURS} hour). Once only one person fits, you get a warrant. The dossiers help you match clues to people.")),
        head("The hideout"),
        body("When witnesses say the thief is still in town, you're close. Keep investigating to catch them, but you need a warrant for the right person or they walk free.".to_string()),
        head("Notes and time"),
        body("The notebook keeps everything witnesses told you. The clock starts on Monday at 9 a.m. and you sleep from 11 p.m. to 7 a.m. Click or press a key to skip the typing.".to_string()),
    ]
}
