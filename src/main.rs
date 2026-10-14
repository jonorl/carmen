//! WHERE IN THE WORLD?  A family-and-friends remake of the classic 1990s
//! detective chase.
//!
//!     cargo run                      play
//!     cargo run -- --check           check your edited data files for mistakes
//!     cargo run -- --seed 42         replay exactly the same case

mod art;
mod device;
mod lang;
mod rules;
mod screens;
mod ui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::prelude::*;

use art::jail::Jail;
use art::map::{Land, MapView};
use art::scene::TimeOfDay;
use art::sprites::Cast;
use art::{map, people, places, scene, sprites};
use carmen_shared::MAX_NAME;
use device::progress::Progress;
use device::sound::Sounds;
use device::{net, progress, sound, touch};
use lang::{Lang, lang, tr};
use rules::data::{Data, LoadError};
use rules::game::{
    Game, INTERVIEW_HOURS, Line, PLACES, RANKS, Tone, WARRANT_HOURS, agency, asleep, clock_text, flight_hours, game_title, hour_of_day,
    promotion_news, rank_for,
};
use rules::rng::Rng;
use rules::{data, game};
use screens::case::*;
use screens::computer;
use screens::hq::*;
use screens::menu::*;
use screens::panel::*;
use screens::printer::Printer;
use screens::windows::*;
use ui::{
    BIG, CYAN, DARK, GREY, INK, Icon, LIME, NAVY, PAPER, RED, SMALL, Ui, YELLOW, draw_pointer, fill, frame, inner, line_height, outline,
    rounded, wrap_cols,
};

const USAGE: &str = "Usage: carmen [--check] [--seed N] [--rank 1-6] [--data FOLDER]

  --check          check the data files and exit
  --seed N         replay the same case by using the same number
  --rank N         play cases as hard as rank N (1 Rookie ... 6 Super Sleuth),
                   whatever your rank; arrests still count towards promotion
  --data FOLDER    folder holding countries.json and suspects.json
                   (default: the current folder if it has them, otherwise the built-in copies)";

struct Args {
    data: Option<PathBuf>,
    seed: Option<u64>,
    rank: Option<usize>,
    check: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { data: None, seed: None, rank: None, check: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value."));
        match arg.as_str() {
            "--check" => args.check = true,
            "--data" => args.data = Some(PathBuf::from(value("--data")?)),
            "--seed" => args.seed = Some(value("--seed")?.parse().map_err(|_| "--seed needs a whole number.")?),
            "--rank" => match value("--rank")?.parse::<usize>() {
                Ok(n @ 1..=6) => args.rank = Some(n - 1),
                _ => return Err("--rank must be 1 to 6.".to_string()),
            },
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("Unknown option {other}.")),
        }
    }
    Ok(args)
}

fn main() {
    let args = parse_args().unwrap_or_else(|msg| {
        eprintln!("{msg}\n\n{USAGE}");
        std::process::exit(2);
    });
    let folder = args.data.clone().or_else(|| {
        let here = PathBuf::from(".");
        here.join("countries.json").exists().then_some(here)
    });
    let (data, warnings) = match data::load(folder.as_deref()) {
        Ok(loaded) => loaded,
        Err(LoadError::Read(msg)) => {
            eprintln!("Problem reading the data files:\n  {msg}");
            std::process::exit(1);
        }
        Err(LoadError::Invalid(errors)) => {
            eprintln!("Please fix these problems in the data files:");
            for e in errors {
                eprintln!("  - {e}");
            }
            std::process::exit(1);
        }
    };
    if args.check {
        let source = match &folder {
            Some(dir) => format!("from {}", dir.display()),
            None => "built into the program".to_string(),
        };
        println!(
            "Data looks good ({source}): {} countries, {} clues, {} suspects, {} traits.",
            data.countries.len(),
            data.clue_count(),
            data.suspects.len(),
            data.traits.len()
        );
        let names: Vec<String> = data.suspects.iter().map(|s| people::photo_name(&s.name)).collect();
        let photos = people::PHOTOS.iter().filter(|(n, _)| names.iter().any(|m| m == n)).count();
        println!("Dossier photos: {photos} of {}.", data.suspects.len());
        for w in warnings {
            println!("  ! {w}");
        }
        for (n, _) in people::PHOTOS.iter().filter(|(n, _)| !names.iter().any(|m| m == n)) {
            println!("  ! assets/suspects/{n}.png doesn't match any suspect's name, so it isn't used.");
        }
        return;
    }
    let conf = Conf {
        window_title: "Where in the World?".to_string(),
        window_width: (WIDE.w * 2.0) as i32,
        window_height: (WIDE.h * 2.0) as i32,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, run(Rc::new(data), args));
}

// ------------------------------------------------------------------ layout

// The screen is laid out like the original: the city and clock top left,
// a picture below it, the text panel on the right and four buttons under it.
// A menu bar runs across the top, as in the original.
const MENU_H: f32 = 16.0;

/// Where everything goes: side by side on wide screens, as in the original,
/// or stacked on tall ones such as phones held upright, with the clock and
/// picture above the text panel and buttons. Each part is the same size in
/// both, so only positions change.
struct Layout {
    /// The size everything is drawn at before scaling to the window.
    w: f32,
    h: f32,
    clock: Rect,
    picture: Rect,
    text: Rect,
    button_y: f32,
    /// The "where to investigate?" window and a suspect's dossier.
    picker: Rect,
    dossier: Rect,
}

const WIDE: Layout = Layout {
    w: 640.0,
    h: 360.0,
    clock: Rect { x: 4.0, y: 18.0, w: 288.0, h: 48.0 },
    picture: Rect { x: 4.0, y: 70.0, w: 288.0, h: 286.0 },
    text: Rect { x: 298.0, y: 18.0, w: 338.0, h: 276.0 },
    button_y: 298.0,
    picker: Rect { x: 130.0, y: 92.0, w: 380.0, h: 204.0 },
    dossier: Rect { x: 34.0, y: 66.0, w: 568.0, h: 282.0 },
};

const TALL: Layout = Layout {
    w: 352.0,
    h: 702.0,
    clock: Rect { x: 32.0, y: 18.0, w: 288.0, h: 48.0 },
    picture: Rect { x: 32.0, y: 70.0, w: 288.0, h: 286.0 },
    text: Rect { x: 7.0, y: 360.0, w: 338.0, h: 276.0 },
    button_y: 640.0,
    picker: Rect { x: 4.0, y: 110.0, w: 344.0, h: 204.0 },
    dossier: Rect { x: 4.0, y: 60.0, w: 344.0, h: 470.0 },
};

thread_local! {
    /// Whether this frame uses `TALL`: chosen from the window's shape each frame.
    static TALL_NOW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn lay() -> &'static Layout {
    if TALL_NOW.get() { &TALL } else { &WIDE }
}

/// The whole drawing area, for clicks anywhere.
fn everywhere() -> Rect {
    Rect::new(0.0, 0.0, lay().w, lay().h)
}
/// Postcards are painted taller than the picture frame; the top of the sky is cropped.
const POSTCARD_H: f32 = 282.0;
const BUTTON_W: f32 = 80.0;
const BUTTON_H: f32 = 58.0;
const BUTTONS: [Icon; 4] = [Icon::Notes, Icon::Depart, Icon::Investigate, Icon::Crime];

// A browser tab can't be closed by the page, so the web build has no Quit.
const CAN_QUIT: bool = !cfg!(target_arch = "wasm32");

/// How long the clock box takes to show each hour passing.
const HOUR_SECONDS: f64 = 0.32;

/// Characters per second for the typewriter effect.
const TYPING_SPEED: f64 = 90.0;
/// The longest headquarters waits for the agency's server before printing
/// the rank from this device's records.
const CHECK_SECONDS: f64 = 4.0;

fn tone_style(tone: Tone) -> (Color, bool) {
    match tone {
        Tone::Plain => (PAPER, false),
        Tone::Dim => (GREY, false),
        Tone::Heading => (YELLOW, true),
        Tone::Good => (LIME, true),
        Tone::Warn => (YELLOW, false),
        Tone::Bad => (RED, true),
    }
}

// ------------------------------------------------------------------- state

enum Screen {
    Hq(Box<Hq>),
    Case(Box<Case>),
}

enum Action {
    Start,
    Continue,
    Show(Panel),
    /// Open or close the "where to investigate?" window.
    Picker(bool),
    /// Walk to place i (0 to 2) in the current city...
    Investigate(usize),
    /// ...and talk to the witness there.
    Arrive(usize),
    /// The chase is over: off to jail, with or without the suspect.
    EndChase,
    /// The walk is over: the jail shows and the printer tells how the case
    /// ended.
    Jail,
    Fly(usize),
    /// Start computing a warrant: the lights flash with the sound...
    Compute,
    /// ...and then the verdict prints.
    Computed,
    ConfirmGiveUp,
    ToTitle,
    Quit,
    // From the menu bar.
    NewCase,
    Help,
    ToggleSound,
    OpenDossier(usize),
    CloseDossier,
    OpenRoster,
    CloseRoster,
    About(bool),
    OpenLink(&'static str),
    Language(Lang),
}

struct App {
    data: Rc<Data>,
    ui: Ui,
    land: Land,
    sounds: Option<Sounds>,
    /// Pictures of the places to investigate, in PLACES order, in English
    /// and in Spanish.
    place_art: [Vec<Texture2D>; 2],
    /// The witness at each place, in PLACES order.
    portraits: Vec<Texture2D>,
    /// Each suspect's dossier photo.
    mugshots: Vec<Texture2D>,
    jail: Jail,
    cast: Cast,
    /// Which menu in the menu bar is open.
    menu: Option<usize>,
    /// The suspect whose dossier is open on top of the screen.
    dossier: Option<usize>,
    /// The detective roster, when it's open on top of the screen.
    roster: Option<Roster>,
    /// Whether the About window is open on top of the screen.
    about: bool,
    /// Sounds decode in the background (asynchronously in browsers), so the
    /// game starts straight away and the typing goes quiet until they're ready.
    loading_sounds: Option<Coroutine<Option<Sounds>>>,
    scenes: HashMap<(usize, TimeOfDay), Texture2D>,
    master: Rng,
    name: String,
    /// Each detective's arrests, kept between games.
    progress: Progress,
    /// From --rank: case difficulty whatever the detective's rank.
    rank_override: Option<usize>,
    screen: Screen,
    quit: bool,
}

async fn run(data: Rc<Data>, args: Args) {
    lang::start();
    let pixel = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/PixelifySans.ttf"));
    let fallback = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/NotoSans-Regular.ttf"));
    let mono = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/VT323.ttf"));
    let (Ok(pixel), Ok(fallback), Ok(mono)) = (pixel, fallback, mono) else {
        eprintln!("Couldn't load the bundled fonts.");
        return;
    };
    let mut app = App {
        data: Rc::clone(&data),
        ui: Ui::new(pixel, fallback, mono),
        land: Land::load(),
        sounds: None,
        place_art: places::pictures(),
        portraits: people::portraits(),
        mugshots: people::suspect_portraits(&data.suspects),
        jail: Jail::new(),
        cast: Cast::new(),
        menu: None,
        dossier: None,
        roster: None,
        about: false,
        loading_sounds: Some(start_coroutine(Sounds::load())),
        scenes: HashMap::new(),
        // std's SystemTime panics in the browser; miniquad's clock works on both.
        master: Rng::new(args.seed.unwrap_or_else(|| (miniquad::date::now() * 1e6) as u64)),
        name: String::new(),
        progress: Progress::load(),
        rank_override: args.rank,
        screen: Screen::Hq(Box::new(Hq::new(None))),
        quit: false,
    };

    // Everything is drawn at the layout's size into one of these textures,
    // then scaled up to the window with hard pixel edges.
    let target = |l: &Layout| {
        let canvas = render_target(l.w as u32, l.h as u32);
        canvas.texture.set_filter(FilterMode::Nearest);
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, l.w, l.h));
        camera.render_target = Some(canvas.clone());
        (canvas, camera)
    };
    let (wide, tall) = (target(&WIDE), target(&TALL));
    show_mouse(false);

    loop {
        let (sw, sh) = (screen_width(), screen_height());
        // A window taller than it's wide (a phone held upright) stacks the halves.
        TALL_NOW.set(sh > sw);
        let ((canvas, camera), l) = if sh > sw { (&tall, &TALL) } else { (&wide, &WIDE) };
        let fit = (sw / l.w).min(sh / l.h);
        // Whole-number scaling keeps every pixel the same size; below 2x
        // (small browser windows) fit exactly instead.
        let scale = if fit >= 2.0 { fit.floor() } else { fit };
        let offset = vec2(((sw - l.w * scale) / 2.0).floor(), ((sh - l.h * scale) / 2.0).floor());

        set_camera(camera);
        clear_background(INK);
        app.ui.begin_frame(offset, scale, l.w);
        app.frame();
        draw_pointer(app.ui.mouse);
        touch::end_frame();

        set_default_camera();
        clear_background(INK);
        // Textures rendered off-screen come out upside down.
        let params = DrawTextureParams { dest_size: Some(vec2(l.w * scale, l.h * scale)), flip_y: true, ..Default::default() };
        draw_texture_ex(&canvas.texture, offset.x, offset.y, WHITE, params);
        if app.quit {
            break;
        }
        next_frame().await;
    }
}

impl App {
    fn frame(&mut self) {
        if let Some(loading) = &self.loading_sounds
            && loading.is_done()
        {
            self.sounds = loading.retrieve().flatten();
            self.loading_sounds = None;
        }
        // The menu bar sits above everything, so it sees the mouse first;
        // while a menu is open the screen underneath ignores input.
        let titles = menu_titles(&self.ui);
        if let Some(i) = (0..titles.len()).find(|&i| self.ui.hovered(titles[i])) {
            if self.ui.take_click(titles[i]) {
                self.menu = if self.menu == Some(i) { None } else { Some(i) };
            } else if self.menu.is_some() {
                self.menu = Some(i);
            }
        }
        // Like the menu titles, the language toggle sees the mouse before the screen.
        if self.menu.is_none()
            && let Some(&(l, r)) = language_buttons(&self.ui).iter().find(|(l, _)| *l != lang())
            && self.ui.take_click(r)
        {
            self.apply(Action::Language(l));
        }
        self.progress.poll();
        // The screen ignores input under an open menu or window.
        self.ui.enabled = self.menu.is_none() && self.dossier.is_none() && self.roster.is_none() && !self.about;
        let mut action = match &mut self.screen {
            Screen::Hq(hq) => hq_screen(&mut self.ui, hq, &mut self.name, &mut self.progress, self.rank_override, &mut self.sounds),
            Screen::Case(case) if case.computing.is_some_and(|t| get_time() - t >= sound::COMPUTE_SECONDS) => Some(Action::Computed),
            Screen::Case(case) if walked(case) => case.walking.map(|(place, _)| Action::Arrive(place)),
            Screen::Case(case) => {
                if let Some(to) = landed(case) {
                    let before = case.game.log.len();
                    let took_off = case.game.t;
                    let hours = flight_hours(case.game.country(case.game.here), case.game.country(to));
                    case.flown = took_off..took_off + hours;
                    case.game.travel(to);
                    case.refresh();
                    let story = case.story_since(before);
                    case.show(Panel::Story(story));
                }
                case.sync_rows(&self.ui, inner(lay().text).w - 4.0);
                let art = Art {
                    places: &self.place_art[usize::from(lang() == Lang::Es)],
                    portraits: &self.portraits,
                    jail: &self.jail,
                    cast: &self.cast,
                };
                case_screen(&mut self.ui, &self.land, &mut self.scenes, art, case, &mut self.sounds)
            }
        };
        self.ui.enabled = self.menu.is_none();
        if let Some(i) = self.dossier {
            let files = Files { data: &self.data, mugshots: &self.mugshots };
            if let Some(chosen) = dossier_window(&mut self.ui, &files, i) {
                action = Some(chosen);
            }
        }
        if let Some(roster) = &mut self.roster {
            roster.poll(&self.progress, &self.name);
            if let Some(chosen) = roster_window(&mut self.ui, roster, &self.name) {
                action = Some(chosen);
            }
        }
        if self.about
            && let Some(chosen) = about_window(&mut self.ui)
        {
            action = Some(chosen);
        }
        self.ui.enabled = true;
        if let Some(chosen) = self.menu_bar(&titles) {
            action = Some(chosen);
        }
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Start => {
                let Screen::Hq(hq) = &mut self.screen else {
                    return;
                };
                let rng = Rng::new(self.master.next_u64());
                let rank = self.rank_override.unwrap_or_else(|| rank_for(self.progress.arrests(&self.name)));
                let mut game = Game::new(Rc::clone(&self.data), RANKS[rank], rng, &self.name);
                let lines = game.briefing();
                hq.printer.finish();
                let level = RANKS[rank].label();
                hq.printer.print(&tr(format!("\n\nAssignment for a {level}:\n"), format!("\n\nMisión de {level}:\n")));
                for (n, line) in lines.iter().filter(|l| l.tone != Tone::Heading).enumerate() {
                    if n > 0 {
                        hq.printer.pause();
                    }
                    hq.printer.print(&format!("\n{}\n", line.text));
                }
                hq.case = Some((game, lines.len()));
                hq.step = Step::Briefing;
            }
            Action::Continue => {
                let Screen::Hq(hq) = &mut self.screen else {
                    return;
                };
                let Some((game, briefed)) = hq.case.take() else {
                    return;
                };
                let arrival = game.log[briefed..].iter().skip_while(|l| l.text.is_empty()).cloned().collect();
                self.screen = Screen::Case(Box::new(Case::new(game, Panel::Story(arrival))));
            }
            Action::ToTitle => {
                if let Some(s) = &mut self.sounds {
                    s.hush();
                }
                let arrests = self.progress.arrests(&self.name);
                self.screen = Screen::Hq(Box::new(Hq::new(Some((&self.name, arrests)))));
            }
            Action::Quit => self.quit = true,
            Action::NewCase => match &mut self.screen {
                // Starting again mid-case means giving this one up first.
                Screen::Case(case) if !case.game.over => case.show(Panel::GiveUp),
                Screen::Case(_) => self.apply(Action::ToTitle),
                Screen::Hq(_) => {}
            },
            Action::Help => match &mut self.screen {
                Screen::Case(case) => {
                    case.picking = false;
                    case.show(Panel::Help);
                }
                Screen::Hq(hq) => {
                    hq.help = true;
                    hq.scroll = 0.0;
                }
            },
            Action::ToggleSound => {
                if let Some(s) = &mut self.sounds {
                    s.on = !s.on;
                }
            }
            Action::OpenDossier(i) => {
                self.roster = None;
                self.dossier = Some(i);
            }
            Action::OpenRoster => {
                self.dossier = None;
                self.roster = Some(Roster::open());
            }
            Action::CloseRoster => self.roster = None,
            Action::About(open) => {
                (self.dossier, self.roster) = (None, None);
                self.about = open;
            }
            Action::OpenLink(url) => net::open_url(url),
            Action::Language(l) => lang::choose(l),
            Action::EndChase => {
                if let Some(s) = &mut self.sounds {
                    s.stop_chase();
                }
                if let Screen::Case(case) = &mut self.screen
                    && let Panel::Chase { then, .. } = &mut case.panel
                {
                    let then = std::mem::take(then);
                    case.show(Panel::March { started: get_time(), then, cued: 0 });
                }
            }
            Action::Jail => {
                if let Screen::Case(case) = &mut self.screen
                    && let Panel::March { then, .. } = &mut case.panel
                {
                    if let Some(s) = &mut self.sounds {
                        s.quiet();
                    }
                    // The first line is the suspect being spotted, which the chase has just shown.
                    let mut ending: Vec<String> = then.iter().skip(1).filter(|l| !l.text.is_empty()).map(printable).collect();
                    // After an arrest, how far the detective is from promotion.
                    ending.extend(case.promotion.clone());
                    case.show(Panel::Jailed { started: get_time(), ending: Some(ending), cued: 0 });
                }
            }
            Action::CloseDossier => self.dossier = None,
            Action::Show(panel) => {
                if let Screen::Case(case) = &mut self.screen {
                    case.show(panel);
                }
            }
            Action::Picker(open) => {
                if let Screen::Case(case) = &mut self.screen {
                    case.picking = open;
                }
            }
            Action::Investigate(i) => {
                let Screen::Case(case) = &mut self.screen else {
                    return;
                };
                if case.game.visited_here().contains(&i) {
                    // Rereading your notes: no need to walk there again.
                    self.apply(Action::Arrive(i));
                    return;
                }
                // The picker stays open while footprints walk to the place.
                case.walking = Some((i, get_time()));
                if let Some(s) = &mut self.sounds {
                    s.walk();
                }
            }
            Action::Compute => {
                let Screen::Case(case) = &mut self.screen else {
                    return;
                };
                if case.game.warrant.iter().all(|(_, v)| v.is_none()) {
                    // Nothing to work on: the computer says so straight away.
                    self.apply(Action::Computed);
                    return;
                }
                case.computing = Some(get_time());
                if let Some(s) = &mut self.sounds {
                    s.compute();
                }
            }
            Action::Arrive(_) | Action::Fly(_) | Action::Computed | Action::ConfirmGiveUp => {
                let Screen::Case(case) = &mut self.screen else {
                    return;
                };
                let before = case.game.log.len();
                match action {
                    Action::Arrive(i) => {
                        if let Some(s) = &mut self.sounds {
                            s.stop_walking();
                        }
                        case.picking = false;
                        case.walking = None;
                        let place = case.places[i];
                        case.game.investigate(i);
                        if case.game.won && case.promotion.is_none() {
                            // Counted (and saved) at the arrest, before the walk to jail.
                            let arrests = self.progress.add_arrest(&self.name);
                            case.promotion = Some(promotion_news(arrests));
                        }
                        let lines = case.story_since(before);
                        // A witness statement gets the portrait and bubble; anything
                        // else (the arrest, the clock running out) is told as a story.
                        let panel = match lines.iter().position(|l| l.speaker.is_some()) {
                            Some(at) if !case.game.over => Panel::Interview {
                                place,
                                said: lines[at].text.trim_matches(['\u{201c}', '\u{201d}']).to_string(),
                                notes: lines
                                    .iter()
                                    .enumerate()
                                    .filter(|&(n, l)| n != at && !l.text.is_empty())
                                    .map(|(_, l)| l.clone())
                                    .collect(),
                            },
                            // Cornered: the chase plays before the arrest is told.
                            _ if case.game.showdown => {
                                if let Some(s) = &mut self.sounds {
                                    s.chase();
                                }
                                Panel::Chase { started: get_time(), then: lines }
                            }
                            _ => Panel::Story(lines),
                        };
                        let interview = matches!(panel, Panel::Interview { .. });
                        let panel = match (case.game.henchman.take(), case.game.danger.take()) {
                            (Some((kind, tell)), _) if interview => {
                                Panel::Henchman { started: get_time(), kind, tell, then: Box::new(panel), cued: 0 }
                            }
                            (_, Some(kind)) if interview => Panel::Danger { started: get_time(), kind, then: Box::new(panel), cued: 0 },
                            _ => panel,
                        };
                        case.show(panel);
                    }
                    Action::Fly(to) => {
                        let from = case.game.here;
                        let hours = flight_hours(case.game.country(from), case.game.country(to));
                        let seconds = (0.8 + hours as f64 * 0.1).min(2.4);
                        case.flight = Some(Flight { from, to, started: get_time(), seconds });
                        case.show(Panel::Flying);
                    }
                    Action::Computed => {
                        // The verdict comes out on the printer, as in the original.
                        case.computing = None;
                        case.game.compute_warrant();
                        case.printer.print("\n");
                        for line in case.story_since(before).iter().filter(|l| !l.text.is_empty()) {
                            case.printer.print(&format!("\n{}", line.text));
                        }
                        case.printer.print(&format!("\n\n{}", ready()));
                    }
                    _ => {
                        case.game.give_up();
                        let story = case.story_since(before);
                        case.show(Panel::Story(story));
                    }
                }
                if case.game.over && !matches!(case.panel, Panel::Story(_) | Panel::Chase { .. }) {
                    let story = case.story_since(before);
                    case.show(Panel::Story(story));
                }
            }
        }
    }
}
