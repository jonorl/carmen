//! WHERE IN THE WORLD?  A family-and-friends remake of the classic 1990s
//! detective chase.
//!
//!     cargo run                      play
//!     cargo run -- --check           check your edited data files for mistakes
//!     cargo run -- --seed 42         replay exactly the same case

mod computer;
mod data;
mod game;
mod jail;
mod landmarks;
mod map;
mod people;
mod pixels;
mod places;
mod printer;
mod rng;
mod scene;
mod sound;
mod sprites;
mod ui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::prelude::*;

use data::{Data, LoadError};
use jail::{Eyes, Jail};
use game::{
    AGENCY, DIFFICULTIES, GAME_TITLE, Game, INTERVIEW_HOURS, Line, PLACES, Tone, WARRANT_HOURS, clock_text, flight_hours,
    hour_of_day,
};
use map::{Land, MapView};
use printer::Printer;
use rng::Rng;
use scene::TimeOfDay;
use sound::Sounds;
use ui::{
    BIG, CYAN, DARK, GREY, H, INK, Icon, LIME, NAVY, PAPER, RED, SMALL, Ui, W, YELLOW, draw_pointer, fill, frame, inner,
    line_height, outline, rounded, wrap_cols,
};

const USAGE: &str = "Usage: carmen [--check] [--seed N] [--difficulty 1|2|3] [--data FOLDER]

  --check          check the data files and exit
  --seed N         replay the same case by using the same number
  --difficulty N   1 Rookie, 2 Detective, 3 Super Sleuth (Enter picks it at headquarters)
  --data FOLDER    folder holding countries.json and suspects.json
                   (default: the current folder if it has them, otherwise the built-in copies)";

struct Args {
    data: Option<PathBuf>,
    seed: Option<u64>,
    difficulty: Option<usize>,
    check: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { data: None, seed: None, difficulty: None, check: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value."));
        match arg.as_str() {
            "--check" => args.check = true,
            "--data" => args.data = Some(PathBuf::from(value("--data")?)),
            "--seed" => args.seed = Some(value("--seed")?.parse().map_err(|_| "--seed needs a whole number.")?),
            "--difficulty" => match value("--difficulty")?.as_str() {
                d @ ("1" | "2" | "3") => args.difficulty = Some(d.parse::<usize>().unwrap_or(1) - 1),
                _ => return Err("--difficulty must be 1, 2 or 3.".to_string()),
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
        window_width: (W * 2.0) as i32,
        window_height: (H * 2.0) as i32,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, run(Rc::new(data), args));
}

// ------------------------------------------------------------------ layout

// The screen is laid out like the original: the city and clock top left,
// a picture below it, the text panel on the right and four buttons under it.
// A menu bar runs across the top, as in the original.
const MENU_H: f32 = 16.0;
const CLOCK: Rect = Rect { x: 4.0, y: 18.0, w: 288.0, h: 48.0 };
const PICTURE: Rect = Rect { x: 4.0, y: 70.0, w: 288.0, h: 286.0 };
const TEXT: Rect = Rect { x: 298.0, y: 18.0, w: 338.0, h: 276.0 };
/// Postcards are painted taller than the picture frame; the top of the sky is cropped.
const POSTCARD_H: f32 = 282.0;
const BUTTON_Y: f32 = 298.0;
const BUTTON_W: f32 = 80.0;
const BUTTON_H: f32 = 58.0;
const BUTTONS: [Icon; 4] = [Icon::Notes, Icon::Depart, Icon::Investigate, Icon::Crime];

// A browser tab can't be closed by the page, so the web build has no Quit.
const CAN_QUIT: bool = !cfg!(target_arch = "wasm32");

/// How long the clock box takes to show each hour passing.
const HOUR_SECONDS: f64 = 0.64;

/// Characters per second for the typewriter effect.
const TYPING_SPEED: f64 = 90.0;

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

struct Flight {
    from: usize,
    to: usize,
    started: f64,
    seconds: f64,
}

/// What the text panel on the right is showing.
enum Panel {
    /// What just happened: an arrival, a night's sleep, an ending.
    Story(Vec<Line>),
    /// The suspect running for it and the police after them, before the
    /// arrest is told (`then`).
    Chase { started: f64, then: Vec<Line> },
    /// With the right warrant: the suspect marched off by an officer...
    March { started: f64, then: Vec<Line> },
    /// ...and locked up, while the end of the case prints beside the jail.
    Jailed { started: f64 },
    /// A witness talking: their portrait, and what they said in a bubble.
    /// `notes` are the other lines the interview produced (the time, a night's sleep).
    Interview { place: usize, said: String, notes: Vec<Line> },
    Depart,
    Flying,
    Notes,
    /// The crime computer's menu, or the list of values for one field.
    Crime { editing: Option<usize> },
    Help,
    GiveUp,
}

/// A wrapped row of the case notes.
struct Row {
    tone: Tone,
    text: String,
    /// Leading bytes of `text` that name the speaker, drawn in another colour.
    speaker: usize,
}

struct Case {
    game: Game,
    /// The printer beside the crime computer, where warrants come out.
    printer: Printer,
    places: [usize; 3],
    dests: Vec<usize>,
    panel: Panel,
    /// When the current text started typing out.
    typed_from: f64,
    flight: Option<Flight>,
    rows: Vec<Row>,
    wrapped: usize,
    notes_scroll: usize,
    page_scroll: f32,
    hover_dest: Option<usize>,
    /// Whether the "where to investigate?" window is open.
    picking: bool,
    /// When the crime computer started on a warrant, while it's working.
    computing: Option<f64>,
    /// Walking to place i (0 to 2) in the picker, and when the walk started.
    walking: Option<(usize, f64)>,
    /// The time the clock box shows, catching up with the game an hour at a time.
    shown_t: i64,
    next_tick: f64,
    /// The departure map, and the city it was made for.
    map: Option<(usize, MapView, Texture2D)>,
}

impl Case {
    fn new(game: Game, panel: Panel) -> Case {
        let mut printer = Printer::new();
        printer.print("READY.");
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
            walking: None,
            shown_t: 0,
            next_tick: 0.0,
            map: None,
        };
        case.refresh();
        case
    }

    /// Places and flights are generated lazily by the game, so ask for them
    /// once per city and keep them for drawing.
    fn refresh(&mut self) {
        let here = self.game.here;
        self.places = self.game.places_in(here);
        self.dests = self.game.destinations();
    }

    fn show(&mut self, panel: Panel) {
        self.panel = panel;
        self.typed_from = get_time();
        self.page_scroll = 0.0;
        self.notes_scroll = 0;
    }

    /// The log lines written since `before`, as a story for the text panel.
    fn story_since(&self, before: usize) -> Vec<Line> {
        self.game.log[before..].iter().skip_while(|l| l.text.is_empty()).cloned().collect()
    }

    fn sync_rows(&mut self, ui: &Ui, width: f32) {
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

/// Where headquarters is up to.
enum Step {
    Name,
    Assignment,
    Briefing,
}

/// The headquarters screen: the printer on the left, prompts on the right.
struct Hq {
    printer: Printer,
    step: Step,
    help: bool,
    scroll: f32,
    /// The case being briefed, with how many log lines the briefing took.
    case: Option<(Game, usize)>,
}

impl Hq {
    fn new(returning: Option<&str>) -> Hq {
        let mut printer = Printer::new();
        printer.print(&format!("{GAME_TITLE}\n\n{}\n\n", AGENCY.to_uppercase()));
        let step = match returning {
            None => {
                printer.print("Welcome to headquarters.\n\nDetective at keyboard, please identify yourself:\n");
                Step::Name
            }
            Some(name) => {
                printer.print(&format!("Welcome back, {}.\n\nChoose your next assignment.", display_name(name)));
                Step::Assignment
            }
        };
        Hq { printer, step, help: false, scroll: 0.0, case: None }
    }
}

fn display_name(name: &str) -> &str {
    match name.trim() {
        "" => "Detective",
        name => name,
    }
}

enum Screen {
    Hq(Box<Hq>),
    Case(Box<Case>),
}

enum Action {
    Start(usize),
    Continue,
    Show(Panel),
    /// Open or close the "where to investigate?" window.
    Picker(bool),
    /// Walk to place i (0 to 2) in the current city...
    Investigate(usize),
    /// ...and talk to the witness there.
    Arrive(usize),
    /// The chase is over: march the suspect off, or tell how it went wrong.
    EndChase,
    /// The march is over: the suspect is behind bars and the printer tells
    /// how the case ended.
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
}

/// The menus across the top of the screen.
const MENUS: [&str; 3] = ["Game", "Options", "Dossiers"];

struct App {
    data: Rc<Data>,
    ui: Ui,
    land: Land,
    sounds: Option<Sounds>,
    /// Pictures of the places to investigate, in PLACES order.
    place_art: Vec<Texture2D>,
    /// The witness at each place, in PLACES order.
    portraits: Vec<Texture2D>,
    /// Each suspect's dossier photo.
    mugshots: Vec<Texture2D>,
    jail: Jail,
    /// Which menu in the menu bar is open.
    menu: Option<usize>,
    /// The suspect whose dossier is open on top of the screen.
    dossier: Option<usize>,
    /// Sounds decode in the background (asynchronously in browsers), so the
    /// game starts straight away and the typing goes quiet until they're ready.
    loading_sounds: Option<Coroutine<Option<Sounds>>>,
    scenes: HashMap<(usize, TimeOfDay), Texture2D>,
    master: Rng,
    name: String,
    preset: Option<usize>,
    screen: Screen,
    quit: bool,
}

async fn run(data: Rc<Data>, args: Args) {
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
        menu: None,
        dossier: None,
        loading_sounds: Some(start_coroutine(Sounds::load())),
        scenes: HashMap::new(),
        // std's SystemTime panics in the browser; miniquad's clock works on both.
        master: Rng::new(args.seed.unwrap_or_else(|| (miniquad::date::now() * 1e6) as u64)),
        name: String::new(),
        preset: args.difficulty,
        screen: Screen::Hq(Box::new(Hq::new(None))),
        quit: false,
    };

    // Everything is drawn at 640x360 into this texture, then scaled up to
    // the window with hard pixel edges.
    let canvas = render_target(W as u32, H as u32);
    canvas.texture.set_filter(FilterMode::Nearest);
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, W, H));
    camera.render_target = Some(canvas.clone());
    show_mouse(false);

    loop {
        let (sw, sh) = (screen_width(), screen_height());
        let fit = (sw / W).min(sh / H);
        // Whole-number scaling keeps every pixel the same size; below 2x
        // (small browser windows) fit exactly instead.
        let scale = if fit >= 2.0 { fit.floor() } else { fit };
        let offset = vec2(((sw - W * scale) / 2.0).floor(), ((sh - H * scale) / 2.0).floor());

        set_camera(&camera);
        clear_background(INK);
        app.ui.begin_frame(offset, scale);
        app.frame();
        draw_pointer(app.ui.mouse);

        set_default_camera();
        clear_background(INK);
        // Textures rendered off-screen come out upside down.
        let params = DrawTextureParams { dest_size: Some(vec2(W * scale, H * scale)), flip_y: true, ..Default::default() };
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
        if let Some(i) = (0..MENUS.len()).find(|&i| self.ui.hovered(titles[i])) {
            if self.ui.take_click(titles[i]) {
                self.menu = if self.menu == Some(i) { None } else { Some(i) };
            } else if self.menu.is_some() {
                self.menu = Some(i);
            }
        }
        // The screen ignores input under an open menu or dossier.
        self.ui.enabled = self.menu.is_none() && self.dossier.is_none();
        let mut action = match &mut self.screen {
            Screen::Hq(hq) => hq_screen(&mut self.ui, hq, &mut self.name, self.preset, &mut self.sounds),
            Screen::Case(case) if case.computing.is_some_and(|t| get_time() - t >= sound::COMPUTE_SECONDS) => {
                Some(Action::Computed)
            }
            Screen::Case(case) if walked(case) => case.walking.map(|(place, _)| Action::Arrive(place)),
            Screen::Case(case) => {
                if let Some(to) = landed(case) {
                    let before = case.game.log.len();
                    case.game.travel(to);
                    case.refresh();
                    let story = case.story_since(before);
                    case.show(Panel::Story(story));
                }
                case.sync_rows(&self.ui, inner(TEXT).w - 4.0);
                let art = Art { places: &self.place_art, portraits: &self.portraits, jail: &self.jail };
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
            Action::Start(d) => {
                let Screen::Hq(hq) = &mut self.screen else { return };
                let rng = Rng::new(self.master.next_u64());
                let mut game = Game::new(Rc::clone(&self.data), DIFFICULTIES[d], rng, &self.name);
                let lines = game.briefing();
                hq.printer.finish();
                hq.printer.print(&format!("\n\nAssignment: {}\n", DIFFICULTIES[d].name));
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
                let Screen::Hq(hq) = &mut self.screen else { return };
                let Some((game, briefed)) = hq.case.take() else { return };
                let arrival = game.log[briefed..].iter().skip_while(|l| l.text.is_empty()).cloned().collect();
                self.screen = Screen::Case(Box::new(Case::new(game, Panel::Story(arrival))));
            }
            Action::ToTitle => {
                if let Some(s) = &mut self.sounds {
                    s.hush();
                }
                self.screen = Screen::Hq(Box::new(Hq::new(Some(&self.name))));
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
            Action::OpenDossier(i) => self.dossier = Some(i),
            Action::EndChase => {
                if let Some(s) = &mut self.sounds {
                    s.stop_chase();
                }
                if let Screen::Case(case) = &mut self.screen
                    && let Panel::Chase { then, .. } = &mut case.panel
                {
                    let then = std::mem::take(then);
                    if case.game.won {
                        if let Some(s) = &mut self.sounds {
                            s.march();
                        }
                        case.show(Panel::March { started: get_time(), then });
                    } else {
                        case.show(Panel::Story(then));
                    }
                }
            }
            Action::Jail => {
                if let Some(s) = &mut self.sounds {
                    s.jail();
                }
                if let Screen::Case(case) = &mut self.screen
                    && let Panel::March { then, .. } = &mut case.panel
                {
                    // The first line is the suspect being spotted, which the chase has just shown.
                    let paras: Vec<String> = then.iter().skip(1).filter(|l| !l.text.is_empty()).map(printable).collect();
                    case.printer = Printer::new();
                    case.printer.print_paragraphs(&paras);
                    case.show(Panel::Jailed { started: get_time() });
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
                let Screen::Case(case) = &mut self.screen else { return };
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
                let Screen::Case(case) = &mut self.screen else { return };
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
                let Screen::Case(case) = &mut self.screen else { return };
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
                        case.printer.print("\n\nREADY.");
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

/// When a flight animation has finished, the city it was heading to.
/// Whether the walk to a witness has finished, so the interview can start.
fn walked(case: &Case) -> bool {
    case.walking.is_some_and(|(_, started)| get_time() - started >= sound::WALK_SECONDS)
}

fn landed(case: &mut Case) -> Option<usize> {
    let f = case.flight.as_ref()?;
    if get_time() - f.started < f.seconds {
        return None;
    }
    let to = f.to;
    case.flight = None;
    Some(to)
}

// ------------------------------------------------------------ headquarters

fn hq_screen(
    ui: &mut Ui,
    hq: &mut Hq,
    name: &mut String,
    preset: Option<usize>,
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

    draw_clock_box(ui, "HEADQUARTERS", &clock_text(0), false);
    frame(PICTURE, PAPER, 3.0);
    hq.printer.draw(ui, inner(PICTURE), naming.then_some(name.as_str()));
    frame(TEXT, PAPER, 3.0);
    let r = inner(TEXT);

    let prompt = if hq.help {
        ui.text("HOW TO PLAY", r.x, r.y + 14.0, BIG, YELLOW, true);
        page(ui, Rect::new(r.x, r.y + 22.0, r.w, r.h - 46.0), &help_paras(), &mut hq.scroll);
        let back = ui.option(Rect::new(r.x, r.y + r.h - 20.0, 70.0, 20.0), "Back", "", true);
        if back || key == Some(KeyCode::Escape) {
            hq.help = false;
        }
        "Scroll to read, then go back."
    } else {
        match hq.step {
            Step::Name => {
                ui.text("IDENTIFY YOURSELF", r.x, r.y + 14.0, BIG, YELLOW, true);
                ui.text("Type your name, then press Enter.", r.x, r.y + 32.0, SMALL, GREY, false);
                let input = Rect::new(r.x, r.y + 40.0, r.w, 22.0);
                fill(input, NAVY);
                outline(input, PAPER);
                let caret = if naming && get_time().fract() < 0.5 { "_" } else { "" };
                ui.text(&format!("{name}{caret}"), input.x + 6.0, input.y + 16.0, BIG, PAPER, true);
                if naming {
                    for c in typed.iter().filter(|c| !c.is_control()) {
                        if name.chars().count() < 20 {
                            name.push(*c);
                            if let Some(s) = sounds {
                                s.click();
                            }
                        }
                    }
                    if key == Some(KeyCode::Backspace) {
                        name.pop();
                    }
                    if matches!(key, Some(KeyCode::Enter | KeyCode::KpEnter)) {
                        hq.printer.type_now(display_name(name));
                        let who = display_name(name).to_string();
                        hq.printer.print(&format!("\n\nYou have been identified, {who}.\n\nChoose your assignment."));
                        hq.step = Step::Assignment;
                    }
                }
                "Type your name and press Enter."
            }
            Step::Assignment => {
                ui.text("CHOOSE YOUR ASSIGNMENT", r.x, r.y + 14.0, BIG, YELLOW, true);
                let spare = ["Time for two wrong turns.", "Time for one wrong turn.", "No time for mistakes."];
                for (i, d) in DIFFICULTIES.iter().enumerate() {
                    let y = r.y + 24.0 + i as f32 * 40.0;
                    if ui.option(Rect::new(r.x, y, r.w, 20.0), d.name, &format!("{} flights", d.legs), true) {
                        return Some(Action::Start(i));
                    }
                    ui.text(spare[i], r.x + 4.0, y + 32.0, SMALL, GREY, false);
                }
                if let Some(d) = preset {
                    ui.text(&format!("Press Enter for {}.", DIFFICULTIES[d].name), r.x, r.y + 150.0, SMALL, GREY, false);
                    if !busy && matches!(key, Some(KeyCode::Enter | KeyCode::KpEnter)) {
                        return Some(Action::Start(d));
                    }
                }
                "Choose an assignment to begin."
            }
            Step::Briefing if busy => "Printing your case file…",
            // Waiting between paragraphs, or done: either way a key carries on.
            Step::Briefing => "Press any key or button to continue.",
        }
    };

    if !hq.help && !matches!(hq.step, Step::Briefing) {
        let footer = r.y + r.h - 20.0;
        if ui.option(Rect::new(r.x, footer, 124.0, 20.0), "How to play", "", true) {
            hq.help = true;
            hq.scroll = 0.0;
        }
        sound_toggle(ui, Rect::new(r.x + 130.0, footer, 112.0, 20.0), sounds);
        if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 54.0, footer, 54.0, 20.0), "Quit", "", true) {
            return Some(Action::Quit);
        }
    }
    prompt_strip(ui, prompt);

    // A key or click anywhere else skips the printing, or carries on once it's done.
    let pressed = (key.is_some() && !naming) || ui.take_click(Rect::new(0.0, 0.0, W, H));
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
fn prompt_strip(ui: &Ui, text: &str) {
    let r = Rect::new(TEXT.x - 2.0, BUTTON_Y - 2.0, W - TEXT.x + 2.0, H - BUTTON_Y - 2.0);
    fill(r, PAPER);
    let rows = ui.wrap(text, BIG, true, r.w - 24.0);
    let mut y = r.y + (r.h - rows.len() as f32 * 19.0) / 2.0 + 14.0;
    for row in rows {
        ui.text_centred(&row, r.x + r.w / 2.0, y, BIG, INK, true);
        y += 19.0;
    }
}

fn sound_toggle(ui: &mut Ui, r: Rect, sounds: &mut Option<Sounds>) {
    if let Some(s) = sounds {
        let label = if s.on { "Sound: on" } else { "Sound: off" };
        if ui.option(r, label, "", true) {
            s.on = !s.on;
        }
    }
}

// -------------------------------------------------------------- the case

fn case_screen(
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
        case.next_tick = get_time() + HOUR_SECONDS;
        if let Some(s) = sounds {
            s.tick();
        }
    }
    draw_clock_box(ui, &game.country(game.here).city, &clock_text(case.shown_t), running_late);

    frame(PICTURE, PAPER, 3.0);
    let picture = inner(PICTURE);
    case.printer.update(get_frame_time(), sounds);
    let mut action = if matches!(case.panel, Panel::Depart | Panel::Flying) {
        draw_map(ui, land, case, picture)
    } else if matches!(case.panel, Panel::Crime { .. } | Panel::Jailed { .. }) {
        case.printer.draw(ui, picture, None);
        None
    } else {
        let game = &case.game;
        let key = (game.here, TimeOfDay::at(hour_of_day(game.t)));
        let texture = scenes
            .entry(key)
            .or_insert_with(|| scene::postcard(game.country(key.0), key.1, picture.w as i32, POSTCARD_H as i32));
        let crop = Rect::new(0.0, POSTCARD_H - picture.h, picture.w, picture.h);
        let params = DrawTextureParams { source: Some(crop), ..Default::default() };
        draw_texture_ex(texture, picture.x, picture.y, WHITE, params);
        None
    };

    frame(TEXT, PAPER, 3.0);
    action = action.or_else(|| text_panel(ui, case, inner(TEXT), art, sounds));

    let busy = case.flight.is_some() || case.game.over || case.computing.is_some();
    let selected = match case.panel {
        _ if case.picking => Some(Icon::Investigate),
        Panel::Interview { .. } => Some(Icon::Investigate),
        Panel::Notes | Panel::Help | Panel::GiveUp => Some(Icon::Notes),
        Panel::Depart | Panel::Flying => Some(Icon::Depart),
        Panel::Crime { .. } => Some(Icon::Crime),
        Panel::Story(_) | Panel::Chase { .. } | Panel::March { .. } | Panel::Jailed { .. } => None,
    };
    for (i, icon) in BUTTONS.into_iter().enumerate() {
        let r = Rect::new(TEXT.x + i as f32 * (BUTTON_W + 6.0), BUTTON_Y, BUTTON_W, BUTTON_H);
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

const PICKER: Rect = Rect { x: 130.0, y: 92.0, w: 380.0, h: 204.0 };

/// The original's "where to?" window: a picture of each place with its
/// name underneath. Hovering either one highlights both.
fn place_picker(ui: &mut Ui, place_art: &[Texture2D], case: &Case) -> Option<Action> {
    frame(PICKER, PAPER, 3.0);
    let r = inner(PICKER);
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
        ui.text_centred(PLACES[p].name, row.x + row.w / 2.0, row.y + 16.0, BIG, colour, true);
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
        return (ui.take_click(Rect::new(0.0, 0.0, W, H)) || ui.any_key()).then_some(Action::Arrive(i));
    }
    let hours = case.game.next_interview_hours();
    let plural = if hours == 1 { "" } else { "s" };
    let hint = format!("A new interview here takes {hours} hour{plural}.");
    ui.text_centred(&hint, r.x + r.w / 2.0, r.y + r.h - 4.0, SMALL, GREY, false);

    if let Some(i) = hovered
        && ui.take_click(PICKER)
    {
        return Some(Action::Investigate(i));
    }
    // Clicking outside the window or pressing Escape closes it.
    let outside = !ui.hovered(PICKER) && ui.take_click(Rect::new(0.0, 0.0, W, H));
    (outside || ui.key(KeyCode::Escape)).then_some(Action::Picker(false))
}

/// The box top left: where you are and when.
fn draw_clock_box(ui: &Ui, place: &str, time: &str, late: bool) {
    frame(CLOCK, PAPER, 3.0);
    let r = inner(CLOCK);
    let cx = r.x + r.w / 2.0;
    ui.text_centred(&ui.fit(place, BIG, true, r.w - 8.0), cx, r.y + 14.0, BIG, PAPER, true);
    ui.text_centred(time, cx, r.y + 31.0, BIG, if late { RED } else { PAPER }, true);
}

fn draw_map(ui: &mut Ui, land: &Land, case: &mut Case, r: Rect) -> Option<Action> {
    let here = case.game.here;
    if case.map.as_ref().is_none_or(|(city, _, _)| *city != here) {
        let points: Vec<(f64, f64)> = std::iter::once(here)
            .chain(case.dests.iter().copied())
            .map(|c| (case.game.country(c).lat, case.game.country(c).lon))
            .collect();
        let view = MapView::fit(r, &points);
        case.map = Some((here, view, view.render(land)));
    }
    let Some((_, view, texture)) = &case.map else { return None };
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
        label(ui, dest, &game.country(f.to).city, YELLOW, r);
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
        label(ui, p, &game.country(d).city, if lit { YELLOW } else { PAPER }, r);
    }
    let p = at(here);
    draw_circle(p.x, p.y, 3.0, INK);
    draw_circle(p.x, p.y, 2.0, if blink { PAPER } else { YELLOW });
    action
}

/// A city name beside its dot, with a black shadow so it reads on any colour.
fn label(ui: &Ui, p: Vec2, text: &str, colour: Color, r: Rect) {
    let w = ui.width(text, SMALL, false);
    let x = if p.x + 5.0 + w < r.x + r.w { p.x + 5.0 } else { p.x - 5.0 - w };
    let y = (p.y + 4.0).clamp(r.y + 10.0, r.y + r.h - 3.0);
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        ui.text(text, x + dx, y + dy, SMALL, INK, false);
    }
    ui.text(text, x, y, SMALL, colour, false);
}

// --------------------------------------------------------- the text panel

/// Pictures drawn once at start-up and shared with the case screen.
#[derive(Clone, Copy)]
struct Art<'a> {
    places: &'a [Texture2D],
    portraits: &'a [Texture2D],
    jail: &'a Jail,
}

fn text_panel(
    ui: &mut Ui,
    case: &mut Case,
    r: Rect,
    art: Art,
    sounds: &mut Option<Sounds>,
) -> Option<Action> {
    let portraits = art.portraits;
    let row = |n: f32| Rect::new(r.x, r.y + n, r.w, 20.0);
    let typed = ((get_time() - case.typed_from) * TYPING_SPEED) as usize;
    let skip = ui.any_key();
    match &case.panel {
        Panel::Story(lines) => {
            let ended = case.game.over;
            let room = if ended { r.h - 24.0 } else { r.h };
            let done = paragraphs(ui, Rect::new(r.x, r.y, r.w, room), lines, typed);
            if !done && (skip || ui.take_click(r)) {
                case.typed_from = f64::NEG_INFINITY;
            }
            if ended && done {
                if ui.option(Rect::new(r.x, r.y + r.h - 20.0, 120.0, 20.0), "New case", "", true) {
                    return Some(Action::ToTitle);
                }
                if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 70.0, r.y + r.h - 20.0, 70.0, 20.0), "Quit", "", true) {
                    return Some(Action::Quit);
                }
            }
            None
        }
        Panel::Chase { started, .. } => {
            chase(ui, r, get_time() - started);
            let over = get_time() - started >= CHASE_SECONDS;
            (over || skip || ui.take_click(r)).then_some(Action::EndChase)
        }
        Panel::March { started, .. } => {
            let t = get_time() - started;
            march(r, t);
            (t >= JAILED_AT || skip || ui.take_click(r)).then_some(Action::Jail)
        }
        Panel::Jailed { started } => {
            let t = get_time() - started;
            art.jail.draw(r, eyes_at(t), t >= GRIP_AT);
            let (busy, waiting) = (case.printer.busy(), case.printer.waiting());
            if busy || waiting {
                if waiting {
                    ui.text("Press a key or click for more.", r.x, r.y + r.h - 6.0, SMALL, GREY, false);
                }
                // The paper is in the picture frame, so a click anywhere carries on.
                if skip || ui.take_click(Rect::new(0.0, MENU_H, W, H - MENU_H)) {
                    if busy {
                        case.printer.finish();
                    } else {
                        case.printer.resume();
                    }
                }
                return None;
            }
            if ui.option(Rect::new(r.x, r.y + r.h - 20.0, 120.0, 20.0), "New case", "", true) {
                return Some(Action::ToTitle);
            }
            if CAN_QUIT && ui.option(Rect::new(r.x + r.w - 70.0, r.y + r.h - 20.0, 70.0, 20.0), "Quit", "", true) {
                return Some(Action::Quit);
            }
            None
        }
        Panel::Interview { place, said, notes } => {
            let done = interview(ui, r, &portraits[*place], PLACES[*place].witness, said, notes, typed);
            if !done && (skip || ui.take_click(r)) {
                case.typed_from = f64::NEG_INFINITY;
            }
            None
        }
        Panel::Depart => {
            let city = &case.game.country(case.game.here).city;
            ui.text(&format!("Flights from {city}:"), r.x, r.y + 14.0, BIG, YELLOW, true);
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
            ui.text("Or click a red dot on the map.", r.x, r.y + 144.0, SMALL, GREY, false);
            None
        }
        Panel::Flying => {
            if let Some(f) = &case.flight {
                let c = case.game.country(f.to);
                ui.text(&format!("Flying to {}…", c.city), r.x, r.y + 14.0, BIG, PAPER, true);
                let hours = flight_hours(case.game.country(f.from), c);
                ui.text(&format!("{hours} hours in the air."), r.x, r.y + 34.0, SMALL, GREY, false);
            }
            None
        }
        Panel::Notes => notes_panel(ui, case, r, sounds),
        Panel::Crime { editing } => {
            let editing = *editing;
            crime_computer(ui, case, r, editing)
        }
        Panel::Help => {
            ui.text("HOW TO PLAY", r.x, r.y + 14.0, BIG, YELLOW, true);
            page(ui, Rect::new(r.x, r.y + 22.0, r.w, r.h - 46.0), &help_paras(), &mut case.page_scroll);
            let back = ui.option(row(r.h - 20.0), "Back to the notes", "", true);
            (back || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Notes))
        }
        Panel::GiveUp => {
            ui.text("Give up this case?", r.x, r.y + 14.0, BIG, YELLOW, true);
            if ui.option(row(28.0), "Yes, give up", "", true) {
                return Some(Action::ConfirmGiveUp);
            }
            let no = ui.option(row(52.0), "No, keep going", "", true);
            (no || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Notes))
        }
    }
}

/// How long the chase lasts: the length of its soundtrack.
const CHASE_SECONDS: f64 = 17.6;

/// The chase, timed to its soundtrack: the suspect runs across the bottom
/// of the panel, "There goes the suspect!", then the police run after them.
fn chase(ui: &Ui, r: Rect, t: f64) {
    let ground = r.y + r.h - 4.0;
    let step = (t / 0.12) as usize;
    // Where a runner of width w is at time t, running across between t0 and t1.
    let across = |t0: f64, t1: f64, w: f32| {
        (t >= t0 && t < t1).then(|| r.x - w + ((t - t0) / (t1 - t0)) as f32 * (r.w + w))
    };
    if let Some(x) = across(1.6, 4.5, sprites::SUSPECT_W) {
        sprites::suspect(x, ground, step, r);
    }
    if (5.7..10.6).contains(&t) {
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        ui.text_centred("There goes", cx, cy, BIG, PAPER, true);
        ui.text_centred("the suspect!", cx, cy + 19.0, BIG, PAPER, true);
    }
    if let Some(x) = across(14.4, 17.0, sprites::POLICE_W) {
        sprites::police(x, ground, step, r);
    }
}

/// The march to jail, timed from the original: the suspect and the officer
/// cross the panel between these times, then after a moment of empty
/// panel the jail appears at `JAILED_AT`.
const MARCH: (f64, f64) = (0.2, 8.6);
const JAILED_AT: f64 = 9.8;
/// When the prisoner grabs the bars, as the jail music ends.
const GRIP_AT: f64 = 14.3;

/// The arrested suspect and the officer walking in from the right and off
/// to the left along the bottom of the panel.
fn march(r: Rect, t: f64) {
    let (t0, t1) = MARCH;
    if !(t0..t1).contains(&t) {
        return;
    }
    let x = r.x + r.w - ((t - t0) / (t1 - t0)) as f32 * (r.w + sprites::MARCH_W);
    sprites::march(x.round(), r.y + r.h - 4.0, (t / 0.25) as usize, r);
}

/// Where the prisoner looks `t` seconds after the door shuts: glancing about
/// at the original's irregular moments, over and over.
fn eyes_at(t: f64) -> Eyes {
    const GLANCES: [(f64, Eyes); 9] = [
        (0.0, Eyes::Ahead),
        (1.5, Eyes::Left),
        (3.8, Eyes::Right),
        (4.4, Eyes::Ahead),
        (6.1, Eyes::Left),
        (8.4, Eyes::Ahead),
        (9.5, Eyes::Right),
        (10.7, Eyes::Ahead),
        (13.0, Eyes::Left),
    ];
    let t = t.rem_euclid(14.6);
    GLANCES.iter().rev().find(|(at, _)| t >= *at).map_or(Eyes::Ahead, |&(_, eyes)| eyes)
}

/// A line of the case as the printer puts it: the paper's font has straight
/// quotes only, and the summary's columns go one under another because the
/// paper is narrow.
fn printable(line: &Line) -> String {
    let text = line.text.replace(['\u{201c}', '\u{201d}'], "\"").replace("   ", "\n");
    match &line.speaker {
        Some(who) => format!("{who}: {text}"),
        None => text,
    }
}

/// A witness portrait bottom left with their job underneath, and what they
/// said typing out in a speech bubble pointing at them. Returns true once
/// the whole statement is showing.
fn interview(ui: &Ui, r: Rect, portrait: &Texture2D, witness: &str, said: &str, notes: &[Line], typed: usize) -> bool {
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
fn paragraphs_height(ui: &Ui, w: f32, lines: &[Line], size: u16, bold: bool) -> f32 {
    let lh = line_height(size);
    lines
        .iter()
        .map(|l| if l.text.is_empty() { 6.0 } else { ui.wrap(&speech(l), size, bold, w).len() as f32 * lh + 3.0 })
        .sum()
}

fn speech(l: &Line) -> String {
    match &l.speaker {
        Some(s) => format!("{s}: {}", l.text),
        None => l.text.clone(),
    }
}

/// Draw log lines as wrapped paragraphs, showing only the first `budget`
/// characters: the typewriter effect. Uses the big font when it fits.
/// Returns true once everything is showing.
fn paragraphs(ui: &Ui, r: Rect, lines: &[Line], budget: usize) -> bool {
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

fn notes_panel(ui: &mut Ui, case: &mut Case, r: Rect, sounds: &mut Option<Sounds>) -> Option<Action> {
    let game = &case.game;
    ui.text("CASE NOTES", r.x, r.y + 14.0, BIG, YELLOW, true);
    let left = (game.deadline - game.t).max(0);
    let deadline = format!("Deadline {} ({left} hours left)", clock_text(game.deadline));
    ui.text(&deadline, r.x, r.y + 30.0, SMALL, if left <= 12 { RED } else { PAPER }, false);
    let w = ui.text("Warrant: ", r.x, r.y + 44.0, SMALL, GREY, false);
    match game.warrant_for {
        Some(s) => ui.text(&game.data.suspects[s].name, r.x + w, r.y + 44.0, SMALL, LIME, true),
        None => ui.text("none", r.x + w, r.y + 44.0, SMALL, GREY, false),
    };

    // The log, newest at the bottom; the mouse wheel scrolls back.
    let log = Rect::new(r.x, r.y + 50.0, r.w, r.h - 76.0);
    fill(Rect::new(r.x, log.y, r.w, 1.0), DARK);
    if ui.hovered(log) && ui.wheel != 0.0 {
        let max = case.rows.len().saturating_sub(4);
        case.notes_scroll =
            if ui.wheel > 0.0 { (case.notes_scroll + 2).min(max) } else { case.notes_scroll.saturating_sub(2) };
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
        let hint = "scroll down for newer";
        ui.text(hint, r.x + r.w - ui.width(hint, SMALL, false), r.y + 14.0, SMALL, GREY, false);
    }
    fill(Rect::new(r.x, log.y + log.h, r.w, 1.0), DARK);

    let footer = r.y + r.h - 20.0;
    if ui.option(Rect::new(r.x, footer, 124.0, 20.0), "How to play", "", true) {
        return Some(Action::Show(Panel::Help));
    }
    sound_toggle(ui, Rect::new(r.x + 130.0, footer, 112.0, 20.0), sounds);
    if ui.option(Rect::new(r.x + r.w - 76.0, footer, 76.0, 20.0), "Give up", "", true) {
        return Some(Action::Show(Panel::GiveUp));
    }
    None
}

/// The crime computer: a menu on the monitor's screen in yellow capitals,
/// the row under the mouse in inverse video. Choosing a field lists its
/// values on the screen; results come out on the printer to the left.
fn crime_computer(ui: &mut Ui, case: &mut Case, r: Rect, editing: Option<usize>) -> Option<Action> {
    let computing = case.computing.map(|started| get_time() - started);
    let screen = computer::draw(r, computing);
    // While it's working the menu shows but can't be used.
    let enabled = ui.enabled;
    ui.enabled = enabled && computing.is_none();
    let action = crime_menu(ui, case, screen, editing);
    ui.enabled = enabled;
    action
}

fn crime_menu(ui: &mut Ui, case: &mut Case, screen: Rect, editing: Option<usize>) -> Option<Action> {
    let s = Rect::new(screen.x + 6.0, screen.y + 5.0, screen.w - 12.0, screen.h - 10.0);
    let fields = case.game.warrant_fields();
    let value_of = |case: &Case, key: &str| {
        case.game.warrant.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.clone())
    };

    let Some(f) = editing else {
        // The menu: each field and its current value, then the commands.
        let rows = fields.len() + 3;
        let row_h = (s.h / (rows as f32 + 0.25)).min(17.0).floor();
        for (i, (key, label, values)) in fields.iter().enumerate() {
            let row = Rect::new(s.x, s.y + i as f32 * row_h, s.w, row_h);
            let value = value_of(case, key).unwrap_or_default();
            if screen_row(ui, row, &format!("{}:", label.to_uppercase()), &value, false) {
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
        if screen_row(ui, row(0.0), &format!("COMPUTE ({WARRANT_HOURS} H)"), "", false) {
            return Some(Action::Compute);
        }
        if screen_row(ui, row(1.0), "CLEAR", "", false) {
            for entry in case.game.warrant.iter_mut() {
                entry.1 = None;
            }
        }
        if screen_row(ui, row(2.0), "DOSSIERS", "", false) {
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
    let mut choices: Vec<(String, Option<String>)> = vec![("DON'T KNOW".to_string(), None)];
    choices.extend(values.iter().map(|v| (v.clone(), Some(v.clone()))));
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
    let back = screen_row(ui, row(choices.len() + 1), "BACK", "", false);
    (back || ui.key(KeyCode::Escape)).then_some(Action::Show(Panel::Crime { editing: None }))
}

/// A line on the computer screen: yellow, or white with blue text under the
/// mouse. `value` follows the label in white; `chosen` marks the current choice.
fn screen_row(ui: &mut Ui, r: Rect, label: &str, value: &str, chosen: bool) -> bool {
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

// ------------------------------------------------------- scrolling pages

struct Para {
    text: String,
    size: u16,
    bold: bool,
    colour: Color,
    gap: f32,
}

fn para(text: impl Into<String>, size: u16, bold: bool, colour: Color, gap: f32) -> Para {
    Para { text: text.into(), size, bold, colour, gap }
}

/// Wrapped paragraphs that scroll with the mouse wheel.
fn page(ui: &Ui, r: Rect, paras: &[Para], scroll: &mut f32) {
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

/// What a dossier needs: the suspects and their photos.
struct Files<'a> {
    data: &'a Data,
    mugshots: &'a [Texture2D],
}

const DOSSIER: Rect = Rect { x: 34.0, y: 66.0, w: 568.0, h: 282.0 };

/// A suspect's file, as in the original: a white sheet over the screen with
/// their name on a tab, their photo in a heavy frame, bold labels with
/// typewritten answers, and their background along the bottom. The arrow
/// keys leaf through the files; a click or Escape closes it.
fn dossier_window(ui: &mut Ui, files: &Files, i: usize) -> Option<Action> {
    let s = &files.data.suspects[i];
    let count = files.data.suspects.len();
    let r = DOSSIER;

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

    let photo = Rect::new(r.x + 14.0, r.y + 14.0, 144.0, 204.0);
    fill(photo, INK);
    outline(Rect::new(photo.x + 3.0, photo.y + 3.0, photo.w - 6.0, photo.h - 6.0), PAPER);
    let params = DrawTextureParams { dest_size: Some(vec2(132.0, 192.0)), ..Default::default() };
    draw_texture_ex(&files.mugshots[i], photo.x + 6.0, photo.y + 6.0, WHITE, params);

    let mut facts: Vec<(&str, &str)> = vec![("Name", &s.name)];
    if files.data.witness_pronouns {
        facts.push(("Pronoun", &s.pronoun));
    }
    for t in &files.data.traits {
        facts.push((&t.label, s.get(&t.key)));
    }
    let (x, right) = (photo.x + photo.w + 14.0, r.x + r.w - 14.0);
    let mut y = r.y + 30.0;
    for (label, value) in facts {
        y = labelled(ui, label, value, (x, right), y);
    }
    if let Some(bio) = &s.bio {
        let y = (y + 2.0).max(photo.y + photo.h + 20.0);
        labelled(ui, "Other", bio, (r.x + 14.0, right), y);
    }
    let hint = format!("File {} of {count}. Left and right arrows: other files. Click: close.", i + 1);
    ui.text_centred(&hint, r.x + r.w / 2.0, r.y + r.h - 8.0, SMALL, DARK, false);

    if ui.key(KeyCode::Left) {
        return Some(Action::OpenDossier((i + count - 1) % count));
    }
    if ui.key(KeyCode::Right) {
        return Some(Action::OpenDossier((i + 1) % count));
    }
    let close = ui.take_click(Rect::new(0.0, MENU_H, W, H)) || ui.key(KeyCode::Escape) || ui.key(KeyCode::Enter);
    close.then_some(Action::CloseDossier)
}

/// Width of a character in the monospaced font.
const MONO_W: f32 = 8.0;

/// "Label: answer" with the label in bold and the answer typewritten,
/// wrapping under the label. Returns the baseline for the next one.
fn labelled(ui: &Ui, label: &str, value: &str, (x, right): (f32, f32), y: f32) -> f32 {
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

// ---------------------------------------------------------------- menu bar

/// Where each menu's title sits in the bar.
fn menu_titles(ui: &Ui) -> Vec<Rect> {
    let mut x = 4.0;
    MENUS
        .iter()
        .map(|title| {
            let w = ui.width(title, BIG, true) + 14.0;
            let r = Rect::new(x, 0.0, w, MENU_H);
            x += w + 4.0;
            r
        })
        .collect()
}

impl App {
    /// The items in menu `m`: a label, and what choosing it does (None when
    /// it can't be chosen right now).
    fn menu_items(&self, m: usize) -> Vec<(String, Option<Action>)> {
        let in_case = matches!(self.screen, Screen::Case(_));
        match m {
            0 => {
                let mut items = vec![
                    ("New case".to_string(), in_case.then_some(Action::NewCase)),
                    ("How to play".to_string(), Some(Action::Help)),
                ];
                if CAN_QUIT {
                    items.push(("Quit".to_string(), Some(Action::Quit)));
                }
                items
            }
            1 => match &self.sounds {
                Some(s) => vec![(format!("Sound: {}", if s.on { "on" } else { "off" }), Some(Action::ToggleSound))],
                None => vec![("Sound: unavailable".to_string(), None)],
            },
            _ => self.data.suspects.iter().enumerate().map(|(i, s)| (s.name.clone(), Some(Action::OpenDossier(i)))).collect(),
        }
    }

    /// Draw the menu bar and any open menu, and return what was chosen.
    fn menu_bar(&mut self, titles: &[Rect]) -> Option<Action> {
        let ui = &self.ui;
        fill(Rect::new(0.0, 0.0, W, MENU_H), PAPER);
        for (i, (title, r)) in MENUS.iter().zip(titles).enumerate() {
            let open = self.menu == Some(i);
            if open {
                fill(*r, INK);
            }
            // The first letter in colour, like the original's keyboard shortcuts.
            let (first, rest) = title.split_at(1);
            let w = ui.text(first, r.x + 7.0, 13.0, BIG, if open { CYAN } else { RED }, true);
            ui.text(rest, r.x + 7.0 + w - 1.0, 13.0, BIG, if open { PAPER } else { INK }, true);
        }
        let m = self.menu?;
        let items = self.menu_items(m);
        let width = items.iter().map(|(label, _)| self.ui.width(label, BIG, true)).fold(0.0, f32::max) + 24.0;
        let rows = items.len() as f32;
        let x = titles[m].x.min(W - width - 2.0);
        let r = Rect::new(x, MENU_H, width, rows * 19.0 + 6.0);
        fill(Rect::new(r.x + 3.0, r.y + 3.0, r.w, r.h), INK);
        fill(r, PAPER);
        outline(r, INK);
        outline(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0), INK);
        let mut chosen = None;
        for (k, (label, action)) in items.into_iter().enumerate() {
            let row = Rect::new(r.x + 3.0, r.y + 3.0 + k as f32 * 19.0, r.w - 6.0, 19.0);
            let active = action.is_some();
            let hover = active && self.ui.hovered(row);
            if hover {
                fill(row, INK);
            }
            let colour = if hover { PAPER } else if active { INK } else { GREY };
            self.ui.text(&label, row.x + 8.0, row.y + 15.0, BIG, colour, true);
            if hover && self.ui.take_click(row) {
                chosen = action;
            }
        }
        // Choosing something, clicking elsewhere or pressing Escape closes it.
        let elsewhere = self.ui.take_click(Rect::new(0.0, 0.0, W, H));
        if chosen.is_some() || elsewhere || is_key_pressed(KeyCode::Escape) {
            self.menu = None;
        }
        chosen
    }
}

fn help_paras() -> Vec<Para> {
    let hours: Vec<String> = INTERVIEW_HOURS.iter().map(|h| h.to_string()).collect();
    let body = |t: String| para(t, SMALL, false, PAPER, 6.0);
    let head = |t: &str| para(t, BIG, true, YELLOW, 0.0);
    vec![
        body("A family member has stolen a treasure and is hopping around the world. Follow the trail to their hideout and arrest them before the deadline.".to_string()),
        head("Investigate"),
        body(format!("The magnifying glass opens a window showing the city's three places, each with a witness; ones you've visited are grey. On the right trail, witnesses hint at the next country (flags, money, landmarks, food, language...) and sometimes describe the thief. In a wrong city nobody has seen a thing, so fly back. Each new interview in a city takes longer ({} hours).", hours.join(", "))),
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
