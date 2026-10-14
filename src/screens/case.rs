//! A case under way: its state, the screen (clock, picture, text panel and
//! buttons), the investigate window and the departure map.

use crate::*;

pub(crate) struct Flight {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) started: f64,
    pub(crate) seconds: f64,
}

/// What the text panel on the right is showing.
pub(crate) enum Panel {
    /// What just happened: an arrival, a night's sleep, an ending.
    Story(Vec<Line>),
    /// The suspect running for it and the police after them, before the
    /// arrest is told (`then`).
    Chase {
        started: f64,
        then: Vec<Line>,
    },
    /// After the chase: the suspect marched off by an officer, or with the
    /// wrong warrant (or none) the officer trudging off alone...
    /// `cued` counts the walk's sound cues played so far.
    March {
        started: f64,
        then: Vec<Line>,
        cued: usize,
    },
    /// ...and the jail, with the suspect in it or empty. `ending` is the end
    /// of the case, until the moment it starts printing beside the jail;
    /// `cued` counts the jail's sound cues played so far.
    Jailed {
        started: f64,
        ending: Option<Vec<String>>,
        cued: usize,
    },
    /// A V.I.L.E. henchman (`kind` 0 to 2) showing himself before the first
    /// interview in a trail city, then the interview (`then`). `tell` adds the
    /// line that says what he means, the first time; `cued` counts his tune.
    Henchman {
        started: f64,
        kind: usize,
        tell: bool,
        then: Box<Panel>,
        cued: usize,
    },
    /// A warning (`kind` 0 to 2: an axe, a knife, a pistol) before a wrong
    /// place's witness in the final city, then the interview (`then`).
    Danger {
        started: f64,
        kind: usize,
        then: Box<Panel>,
        cued: usize,
    },
    /// A witness talking: their portrait, and what they said in a bubble.
    /// `notes` are the other lines the interview produced (the time, a night's sleep).
    Interview {
        place: usize,
        said: String,
        notes: Vec<Line>,
    },
    Depart,
    Flying,
    Notes,
    /// The crime computer's menu, or the list of values for one field.
    Crime {
        editing: Option<usize>,
    },
    Help,
    GiveUp,
}

/// A wrapped row of the case notes.
pub(crate) struct Row {
    pub(crate) tone: Tone,
    pub(crate) text: String,
    /// Leading bytes of `text` that name the speaker, drawn in another colour.
    pub(crate) speaker: usize,
}

pub(crate) struct Case {
    pub(crate) game: Game,
    /// After an arrest: what the printer says about the detective's promotion.
    pub(crate) promotion: Option<String>,
    /// The printer beside the crime computer, where warrants come out.
    pub(crate) printer: Printer,
    pub(crate) places: [usize; 3],
    pub(crate) dests: Vec<usize>,
    pub(crate) panel: Panel,
    /// When the current text started typing out.
    pub(crate) typed_from: f64,
    pub(crate) flight: Option<Flight>,
    pub(crate) rows: Vec<Row>,
    pub(crate) wrapped: usize,
    pub(crate) notes_scroll: usize,
    pub(crate) page_scroll: f32,
    pub(crate) hover_dest: Option<usize>,
    /// Whether the "where to investigate?" window is open.
    pub(crate) picking: bool,
    /// When the crime computer started on a warrant, while it's working.
    pub(crate) computing: Option<f64>,
    /// Walking to place i (0 to 2) in the picker, and when the walk started.
    pub(crate) walking: Option<(usize, f64)>,
    /// The time the clock box shows, catching up with the game an hour at a time.
    pub(crate) shown_t: i64,
    pub(crate) next_tick: f64,
    /// The hours spent on the last flight, which the clock runs through quickly.
    pub(crate) flown: std::ops::Range<i64>,
    /// The departure map, and the city it was made for.
    pub(crate) map: Option<(usize, MapView, Texture2D)>,
}

/// What the crime computer's printer says when it's waiting.
pub(crate) fn ready() -> &'static str {
    tr("READY.", "LISTO.")
}

impl Case {
    pub(crate) fn new(game: Game, panel: Panel) -> Case {
        let mut printer = Printer::new();
        printer.print(ready());
        printer.finish();
        let mut case = Case {
            game,
            printer,
            places: [0; 3],
            dests: Vec::new(),
            panel,
            typed_from: get_time(),
            flight: None,
            rows: Vec::new(),
            wrapped: 0,
            notes_scroll: 0,
            page_scroll: 0.0,
            hover_dest: None,
            picking: false,
            computing: None,
            promotion: None,
            walking: None,
            shown_t: 0,
            next_tick: 0.0,
            flown: 0..0,
            map: None,
        };
        case.refresh();
        case
    }

    /// Places and flights are generated lazily by the game, so ask for them
    /// once per city and keep them for drawing.
    pub(crate) fn refresh(&mut self) {
        let here = self.game.here;
        self.places = self.game.places_in(here);
        self.dests = self.game.destinations();
    }

    pub(crate) fn show(&mut self, panel: Panel) {
        self.panel = panel;
        self.typed_from = get_time();
        self.page_scroll = 0.0;
        self.notes_scroll = 0;
    }

    /// The log lines written since `before`, as a story for the text panel.
    pub(crate) fn story_since(&self, before: usize) -> Vec<Line> {
        self.game.log[before..].iter().skip_while(|l| l.text.is_empty()).cloned().collect()
    }

    pub(crate) fn sync_rows(&mut self, ui: &Ui, width: f32) {
        for line in &self.game.log[self.wrapped..] {
            let (_, bold) = tone_style(line.tone);
            let (text, speaker) = match &line.speaker {
                Some(s) => (format!("{s}: {}", line.text), s.len() + 1),
                None => (line.text.clone(), 0),
            };
            for (n, row) in ui.wrap(&text, SMALL, bold, width).into_iter().enumerate() {
                let speaker = if n == 0 { speaker.min(row.len()) } else { 0 };
                self.rows.push(Row { tone: line.tone, text: row, speaker });
            }
        }
        self.wrapped = self.game.log.len();
    }
}

/// When a flight animation has finished, the city it was heading to.
/// Whether the walk to a witness has finished, so the interview can start.
pub(crate) fn walked(case: &Case) -> bool {
    case.walking.is_some_and(|(_, started)| get_time() - started >= sound::WALK_SECONDS)
}

pub(crate) fn landed(case: &mut Case) -> Option<usize> {
    let f = case.flight.as_ref()?;
    if get_time() - f.started < f.seconds {
        return None;
    }
    let to = f.to;
    case.flight = None;
    Some(to)
}

pub(crate) fn case_screen(
    ui: &mut Ui,
    land: &Land,
    scenes: &mut HashMap<(usize, TimeOfDay), Texture2D>,
    art: Art,
    case: &mut Case,
    sounds: &mut Option<Sounds>,
) -> Option<Action> {
    // With the picker open, the screen underneath is drawn but ignores input.
    let enabled = ui.enabled;
    ui.enabled = enabled && !case.picking;
    let game = &case.game;
    let running_late = !game.over && game.deadline - game.t <= 12;
    // The clock catches up an hour at a time, ticking for each one, always at the same pace.
    if case.shown_t > game.t {
        case.shown_t = game.t;
    }
    if case.shown_t < game.t && get_time() >= case.next_tick {
        case.shown_t += 1;
        // Hours asleep or in the air go by twice as fast.
        let quick = asleep(case.shown_t) || case.flown.contains(&case.shown_t);
        case.next_tick = get_time() + if quick { HOUR_SECONDS / 2.0 } else { HOUR_SECONDS };
        if let Some(s) = sounds {
            s.tick();
        }
    }
    draw_clock_box(ui, game.country(game.here).city(), &clock_text(case.shown_t), running_late);

    frame(lay().picture, PAPER, 3.0);
    let picture = inner(lay().picture);
    case.printer.update(get_frame_time(), sounds);
    if let Some(s) = sounds {
        // From take-off until landing, when the map gives way to the city.
        s.travelling(case.flight.is_some());
    }
    let mut action = if matches!(case.panel, Panel::Depart | Panel::Flying) {
        draw_map(ui, land, case, picture)
    } else if match &case.panel {
        Panel::Crime { .. } => true,
        Panel::Jailed { started, .. } => get_time() - started >= print_at(case.game.won),
        _ => false,
    } {
        case.printer.draw(ui, picture, None);
        None
    } else {
        let game = &case.game;
        let key = (game.here, TimeOfDay::at(hour_of_day(game.t)));
        let texture = scenes.entry(key).or_insert_with(|| scene::postcard(game.country(key.0), key.1, picture.w as i32, POSTCARD_H as i32));
        let crop = Rect::new(0.0, POSTCARD_H - picture.h, picture.w, picture.h);
        let params = DrawTextureParams { source: Some(crop), ..Default::default() };
        draw_texture_ex(texture, picture.x, picture.y, WHITE, params);
        None
    };

    frame(lay().text, PAPER, 3.0);
    action = action.or_else(|| text_panel(ui, case, inner(lay().text), art, sounds));

    let busy = case.flight.is_some()
        || case.game.over
        || case.computing.is_some()
        || matches!(case.panel, Panel::Henchman { .. } | Panel::Danger { .. });
    let selected = match case.panel {
        _ if case.picking => Some(Icon::Investigate),
        Panel::Interview { .. } | Panel::Henchman { .. } | Panel::Danger { .. } => Some(Icon::Investigate),
        Panel::Notes | Panel::Help | Panel::GiveUp => Some(Icon::Notes),
        Panel::Depart | Panel::Flying => Some(Icon::Depart),
        Panel::Crime { .. } => Some(Icon::Crime),
        Panel::Story(_) | Panel::Chase { .. } | Panel::March { .. } | Panel::Jailed { .. } => None,
    };
    for (i, icon) in BUTTONS.into_iter().enumerate() {
        let r = Rect::new(lay().text.x + i as f32 * (BUTTON_W + 6.0), lay().button_y, BUTTON_W, BUTTON_H);
        if ui.icon_button(r, icon, selected == Some(icon), !busy) {
            action = Some(match icon {
                Icon::Notes => Action::Show(Panel::Notes),
                Icon::Depart => Action::Show(Panel::Depart),
                Icon::Investigate => Action::Picker(true),
                Icon::Crime => Action::Show(Panel::Crime { editing: None }),
            });
        }
    }

    ui.enabled = enabled;
    if case.picking {
        action = place_picker(ui, art.places, case);
    }
    action
}

/// The original's "where to?" window: a picture of each place with its
/// name underneath. Hovering either one highlights both.
pub(crate) fn place_picker(ui: &mut Ui, place_art: &[Texture2D], case: &Case) -> Option<Action> {
    frame(lay().picker, PAPER, 3.0);
    let r = inner(lay().picker);
    let art_w = places::ART_W as f32;
    let gap = (r.w - 3.0 * art_w) / 4.0;
    let pictures: Vec<Rect> =
        (0..3).map(|i| Rect::new(r.x + gap + i as f32 * (art_w + gap), r.y + 8.0, art_w, places::ART_H as f32)).collect();
    // Room between the pictures and the names for footprints to walk up.
    let names: Vec<Rect> = (0..3).map(|i| Rect::new(r.x + r.w / 2.0 - 100.0, r.y + 108.0 + i as f32 * 22.0, 200.0, 21.0)).collect();
    let walking = case.walking;
    let hovered = match walking {
        Some((i, _)) => Some(i),
        None => (0..3).find(|&i| ui.hovered(pictures[i]) || ui.hovered(names[i])),
    };

    for (i, &p) in case.places.iter().enumerate() {
        let pic = pictures[i];
        draw_texture(&place_art[p], pic.x, pic.y, WHITE);
        let visited = case.game.visited_here().contains(&i);
        let row = names[i];
        let colour = if hovered == Some(i) {
            fill(row, PAPER);
            outline(Rect::new(pic.x - 3.0, pic.y - 3.0, pic.w + 6.0, pic.h + 6.0), YELLOW);
            INK
        } else if visited {
            GREY
        } else {
            PAPER
        };
        ui.text_centred(PLACES[p].label(), row.x + row.w / 2.0, row.y + 16.0, BIG, colour, true);
    }
    if let Some((i, started)) = walking {
        // Footprints appear one by one, left and right in turn, from the list
        // up to the chosen place, in time with the footsteps.
        const STEPS: usize = 8;
        let shown = ((get_time() - started) / sound::WALK_SECONDS * STEPS as f64) as usize + 1;
        let from = vec2(r.x + r.w / 2.0 - 30.0, names[0].y - 4.0);
        let to = vec2(pictures[i].x + pictures[i].w / 2.0, pictures[i].y + pictures[i].h + 6.0);
        let dir = (to - from).normalize_or_zero();
        let side = vec2(-dir.y, dir.x);
        for n in 0..shown.min(STEPS) {
            let t = n as f32 / (STEPS - 1) as f32;
            let p = from.lerp(to, t) + side * if n % 2 == 0 { 2.5 } else { -2.5 };
            let (x, y) = (p.x.round(), p.y.round());
            fill(Rect::new(x, y, 2.0, 2.0), PAPER);
            fill(Rect::new(x + dir.x.signum(), y - 2.0, 1.0, 1.0), PAPER);
        }
        // A click or key gets there at once.
        return (ui.take_click(everywhere()) || ui.any_key()).then_some(Action::Arrive(i));
    }
    let hint =
        tr(format!("A new interview here takes {INTERVIEW_HOURS} hours."), format!("Cada entrevista nueva lleva {INTERVIEW_HOURS} horas."));
    ui.text_centred(&hint, r.x + r.w / 2.0, r.y + r.h - 4.0, SMALL, GREY, false);

    if let Some(i) = hovered
        && ui.take_click(lay().picker)
    {
        return Some(Action::Investigate(i));
    }
    // Clicking outside the window or pressing Escape closes it.
    let outside = !ui.hovered(lay().picker) && ui.take_click(everywhere());
    (outside || ui.key(KeyCode::Escape)).then_some(Action::Picker(false))
}

/// The box top left: where you are and when.
pub(crate) fn draw_clock_box(ui: &Ui, place: &str, time: &str, late: bool) {
    frame(lay().clock, PAPER, 3.0);
    let r = inner(lay().clock);
    let cx = r.x + r.w / 2.0;
    ui.text_centred(&ui.fit(place, BIG, true, r.w - 8.0), cx, r.y + 14.0, BIG, PAPER, true);
    ui.text_centred(time, cx, r.y + 31.0, BIG, if late { RED } else { PAPER }, true);
}

pub(crate) fn draw_map(ui: &mut Ui, land: &Land, case: &mut Case, r: Rect) -> Option<Action> {
    let here = case.game.here;
    if case.map.as_ref().is_none_or(|(city, _, _)| *city != here) {
        let points: Vec<(f64, f64)> =
            std::iter::once(here).chain(case.dests.iter().copied()).map(|c| (case.game.country(c).lat, case.game.country(c).lon)).collect();
        let view = MapView::fit(r, &points);
        case.map = Some((here, view, view.render(land)));
    }
    let Some((_, view, texture)) = &case.map else {
        return None;
    };
    draw_texture(texture, r.x, r.y, WHITE);
    let game = &case.game;
    let at = |c: usize| view.project(game.country(c).lat, game.country(c).lon);

    for pair in game.path.windows(2) {
        if let Some((a, b)) = map::clip(at(pair[0]), at(pair[1]), r) {
            draw_line(a.x, a.y, b.x, b.y, 1.0, YELLOW);
        }
    }
    let blink = get_time().fract() < 0.5;
    if let Some(f) = &case.flight {
        let (a, b) = (at(f.from), at(f.to));
        let t = ((get_time() - f.started) / f.seconds).clamp(0.0, 1.0) as f32;
        let pos = a.lerp(b, t * t * (3.0 - 2.0 * t));
        if let Some((a, p)) = map::clip(a, pos, r) {
            draw_line(a.x, a.y, p.x, p.y, 1.0, PAPER);
        }
        let dir = (b - a).normalize_or_zero();
        let side = vec2(-dir.y, dir.x);
        draw_triangle(pos + dir * 6.0, pos - dir * 4.0 + side * 4.0, pos - dir * 4.0 - side * 4.0, PAPER);
        let dest = at(f.to);
        draw_circle(dest.x, dest.y, 3.0, INK);
        draw_circle(dest.x, dest.y, 2.0, YELLOW);
        label(ui, dest, game.country(f.to).city(), YELLOW, r);
        return None;
    }

    // Destinations: red dots with names. Hover or click one to fly there.
    let mut hovered = None;
    for &d in &case.dests {
        let p = at(d);
        if ui.enabled && p.distance(ui.mouse) < 7.0 {
            hovered = Some(d);
        }
    }
    let mut action = None;
    if let Some(d) = hovered {
        case.hover_dest = Some(d);
        if ui.take_click(r) {
            action = Some(Action::Fly(d));
        }
    }
    for &d in &case.dests {
        let p = at(d);
        let lit = case.hover_dest == Some(d);
        draw_circle(p.x, p.y, 3.0, INK);
        draw_circle(p.x, p.y, 2.0, if lit { YELLOW } else { RED });
        label(ui, p, game.country(d).city(), if lit { YELLOW } else { PAPER }, r);
    }
    let p = at(here);
    draw_circle(p.x, p.y, 3.0, INK);
    draw_circle(p.x, p.y, 2.0, if blink { PAPER } else { YELLOW });
    action
}

/// A city name beside its dot, with a black shadow so it reads on any colour.
pub(crate) fn label(ui: &Ui, p: Vec2, text: &str, colour: Color, r: Rect) {
    let w = ui.width(text, SMALL, false);
    let x = if p.x + 5.0 + w < r.x + r.w { p.x + 5.0 } else { p.x - 5.0 - w };
    let y = (p.y + 4.0).clamp(r.y + 10.0, r.y + r.h - 3.0);
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        ui.text(text, x + dx, y + dy, SMALL, INK, false);
    }
    ui.text(text, x, y, SMALL, colour, false);
}
