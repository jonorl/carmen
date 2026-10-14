//! Headquarters: the printer on the left, where the detective signs in and
//! the case is briefed, and the prompts on the right.

use crate::*;

/// Where headquarters is up to.
pub(crate) enum Step {
    Name,
    /// Identified, and waiting (since the time given) for the agency's
    /// server to say what rank they've reached.
    Checking(f64),
    Assignment,
    Briefing,
}

/// The headquarters screen: the printer on the left, prompts on the right.
pub(crate) struct Hq {
    pub(crate) printer: Printer,
    pub(crate) step: Step,
    pub(crate) help: bool,
    pub(crate) scroll: f32,
    /// The case being briefed, with how many log lines the briefing took.
    pub(crate) case: Option<(Game, usize)>,
}

impl Hq {
    /// Headquarters, with `returning` the detective coming back from a case
    /// and their arrests so far.
    pub(crate) fn new(returning: Option<(&str, u32)>) -> Hq {
        let mut printer = Printer::new();
        printer.print(&format!("{}\n\n{}\n\n", game_title(), agency().to_uppercase()));
        let step = match returning {
            None => {
                printer.print(tr(
                    "Welcome to headquarters.\n\nDetective at keyboard, please identify yourself:\n",
                    "Cuartel general.\n\nDetective al teclado, identificate:\n",
                ));
                Step::Name
            }
            Some((name, arrests)) => {
                let (who, rank) = (display_name(name), RANKS[rank_for(arrests)].label());
                printer.print(&tr(
                    format!("Welcome back, {who}.\n\nYour rank: {rank}."),
                    format!("Hola de nuevo, {who}.\n\nTu rango: {rank}."),
                ));
                Step::Assignment
            }
        };
        Hq { printer, step, help: false, scroll: 0.0, case: None }
    }
}

pub(crate) fn display_name(name: &str) -> &str {
    match name.trim() {
        "" => "Detective",
        name => name,
    }
}

pub(crate) fn hq_screen(
    ui: &mut Ui,
    hq: &mut Hq,
    name: &mut String,
    progress: &mut Progress,
    rank_override: Option<usize>,
    sounds: &mut Option<Sounds>,
) -> Option<Action> {
    hq.printer.update(get_frame_time(), sounds);
    let mut key = get_last_key_pressed();
    let mut typed = Vec::new();
    while let Some(c) = get_char_pressed() {
        typed.push(c);
    }
    if !ui.enabled {
        // A menu is open: keys belong to it.
        key = None;
        typed.clear();
    }
    let busy = hq.printer.busy();
    let waiting = hq.printer.waiting();
    let naming = matches!(hq.step, Step::Name) && !busy && !waiting && !hq.help;

    draw_clock_box(ui, tr("HEADQUARTERS", "CUARTEL GENERAL"), &clock_text(0), false);
    frame(lay().picture, PAPER, 3.0);
    hq.printer.draw(ui, inner(lay().picture), naming.then_some(name.as_str()));
    frame(lay().text, PAPER, 3.0);
    let r = inner(lay().text);

    let prompt = if hq.help {
        ui.text(tr("HOW TO PLAY", "CÓMO JUGAR"), r.x, r.y + 14.0, BIG, YELLOW, true);
        page(ui, Rect::new(r.x, r.y + 22.0, r.w, r.h - 46.0), &help_paras(), &mut hq.scroll);
        let back = ui.option(Rect::new(r.x, r.y + r.h - 20.0, 70.0, 20.0), tr("Back", "Volver"), "", true);
        if back || key == Some(KeyCode::Escape) {
            hq.help = false;
        }
        tr("Scroll to read, then go back.", "Deslizá para leer y después volvé.")
    } else {
        match hq.step {
            Step::Name => {
                ui.text(tr("IDENTIFY YOURSELF", "IDENTIFICATE"), r.x, r.y + 14.0, BIG, YELLOW, true);
                ui.text(tr("Type your name, then press Enter.", "Escribí tu nombre y apretá Enter."), r.x, r.y + 32.0, SMALL, GREY, false);
                let input = Rect::new(r.x, r.y + 40.0, r.w, 22.0);
                fill(input, NAVY);
                outline(input, PAPER);
                let caret = if naming && get_time().fract() < 0.5 { "_" } else { "" };
                ui.text(&format!("{name}{caret}"), input.x + 6.0, input.y + 16.0, BIG, PAPER, true);
                if naming {
                    // On phones a real text box covers the field, for their keyboards.
                    touch::want(ui.to_window(input));
                    if let Some(text) = touch::typed() {
                        *name = text.chars().take(MAX_NAME).collect();
                    }
                    for c in typed.iter().filter(|c| !c.is_control()) {
                        if name.chars().count() < MAX_NAME {
                            name.push(*c);
                            if let Some(s) = sounds {
                                s.click();
                            }
                        }
                    }
                    if key == Some(KeyCode::Backspace) {
                        name.pop();
                    }
                    if matches!(key, Some(KeyCode::Enter | KeyCode::KpEnter)) || touch::entered() {
                        hq.printer.type_now(display_name(name));
                        let who = display_name(name).to_string();
                        hq.printer.print(&tr(format!("\n\nYou have been identified, {who}."), format!("\n\nIdentidad confirmada, {who}.")));
                        progress.look_up(name);
                        hq.step = Step::Checking(get_time());
                    }
                }
                tr("Type your name and press Enter.", "Escribí tu nombre y apretá Enter.")
            }
            Step::Checking(since) => {
                ui.text(tr("CHECKING RECORDS", "REVISANDO ARCHIVOS"), r.x, r.y + 14.0, BIG, YELLOW, true);
                // Rather a stale rank than a long wait: a late answer still
                // counts, it just isn't printed.
                let given_up = get_time() - since > CHECK_SECONDS;
                if !busy && (!progress.looking_up(name) || given_up) {
                    let rank = RANKS[rank_for(progress.arrests(name))].label();
                    hq.printer.print(&tr(format!("\n\nYour rank: {rank}."), format!("\n\nTu rango: {rank}.")));
                    hq.step = Step::Assignment;
                }
                tr("Checking the agency's records…", "Revisando los archivos de la agencia…")
            }
            Step::Assignment => {
                ui.text(tr("YOUR RANK", "TU RANGO"), r.x, r.y + 14.0, BIG, YELLOW, true);
                let arrests = progress.arrests(name);
                let rank = rank_for(arrests);
                let plural = |n: u32| if n == 1 { "" } else { "s" };
                ui.text(RANKS[rank].label(), r.x, r.y + 36.0, BIG, PAPER, true);
                let so_far = tr(
                    format!("{arrests} arrest{} so far.", plural(arrests)),
                    format!("{arrests} arresto{} hasta ahora.", plural(arrests)),
                );
                ui.text(&so_far, r.x, r.y + 56.0, SMALL, GREY, false);
                if let Some(next) = RANKS.get(rank + 1) {
                    let (n, to) = (next.arrests - arrests, next.label());
                    ui.text(
                        &tr(format!("{n} more for promotion to {to}."), format!("{n} más para ascender a {to}.")),
                        r.x,
                        r.y + 72.0,
                        SMALL,
                        GREY,
                        false,
                    );
                } else {
                    ui.text(tr("The agency's highest rank.", "El rango más alto de la agencia."), r.x, r.y + 72.0, SMALL, GREY, false);
                }
                // Each rank's cases are longer, or leave less time for wrong turns.
                let case = &RANKS[rank_override.unwrap_or(rank)];
                let spare = tr(
                    ["No time for mistakes.", "Time for one wrong turn.", "Time for two wrong turns."],
                    ["Sin tiempo para errores.", "Tiempo para un desvío.", "Tiempo para dos desvíos."],
                );
                let (legs, spare) = (case.legs, spare[case.spare_detours.min(2)]);
                ui.text(
                    &tr(format!("Cases: {legs} flights. {spare}"), format!("Casos: {legs} vuelos. {spare}")),
                    r.x,
                    r.y + 96.0,
                    SMALL,
                    GREY,
                    false,
                );
                if rank_override.is_some() {
                    let level = case.label();
                    ui.text(
                        &tr(format!("(Playing {level} cases, from --rank.)"), format!("(Casos de {level}, por --rank.)")),
                        r.x,
                        r.y + 112.0,
                        SMALL,
                        GREY,
                        false,
                    );
                }
                let take =
                    ui.option(Rect::new(r.x, r.y + 126.0, r.w, 20.0), tr("Take your next case", "Tomá tu próximo caso"), "Enter", true);
                if take || (!busy && matches!(key, Some(KeyCode::Enter | KeyCode::KpEnter))) {
                    return Some(Action::Start);
                }
                tr("Take your next case when you're ready.", "Tomá tu próximo caso cuando quieras.")
            }
            Step::Briefing if busy => tr("Printing your case file…", "Imprimiendo el expediente…"),
            // Waiting between paragraphs, or done: either way a key carries on.
            Step::Briefing => tr("Press any key or button to continue.", "Apretá cualquier tecla o botón para seguir."),
        }
    };

    if !hq.help && !matches!(hq.step, Step::Briefing) {
        let footer = r.y + r.h - 20.0;
        if ui.option(Rect::new(r.x, footer, 124.0, 20.0), tr("How to play", "Cómo jugar"), "", true) {
            hq.help = true;
            hq.scroll = 0.0;
        }
        sound_toggle(ui, Rect::new(r.x + 130.0, footer, 112.0, 20.0), sounds);
        if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 54.0, footer, 54.0, 20.0), tr("Quit", "Salir"), "", true) {
            return Some(Action::Quit);
        }
    }
    prompt_strip(ui, prompt);

    // A key or click anywhere else skips the printing, or carries on once it's done.
    let pressed = (key.is_some() && !naming) || ui.take_click(everywhere());
    if pressed && busy {
        hq.printer.finish();
    } else if pressed && waiting {
        hq.printer.resume();
    } else if pressed && matches!(hq.step, Step::Briefing) && !hq.help {
        return Some(Action::Continue);
    }
    None
}

/// The white strip under the text panel that tells the player what to do.
pub(crate) fn prompt_strip(ui: &Ui, text: &str) {
    let r = Rect::new(lay().text.x - 2.0, lay().button_y - 2.0, lay().w - lay().text.x + 2.0, lay().h - lay().button_y - 2.0);
    fill(r, PAPER);
    let rows = ui.wrap(text, BIG, true, r.w - 24.0);
    let mut y = r.y + (r.h - rows.len() as f32 * 19.0) / 2.0 + 14.0;
    for row in rows {
        ui.text_centred(&row, r.x + r.w / 2.0, y, BIG, INK, true);
        y += 19.0;
    }
}

pub(crate) fn sound_label(on: bool) -> &'static str {
    match (on, lang()) {
        (true, Lang::En) => "Sound: on",
        (false, Lang::En) => "Sound: off",
        (true, Lang::Es) => "Sonido: sí",
        (false, Lang::Es) => "Sonido: no",
    }
}

pub(crate) fn sound_toggle(ui: &mut Ui, r: Rect, sounds: &mut Option<Sounds>) {
    if let Some(s) = sounds {
        let label = sound_label(s.on);
        if ui.option(r, label, "", true) {
            s.on = !s.on;
        }
    }
}
