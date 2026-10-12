# Where in the World? (family edition)

A remake of the 1990s DOS game "Where in the World is Carmen Sandiego?". The thieves are the player's family and friends instead of V.I.L.E. henchmen, and the country facts are updated. Written in Rust with macroquad, styled after the 1990s DOS version (black screen, double-line white frames, pixel font, a picture on the left, a text panel on the right, four icon buttons). A recorded printer sound plays while the printer prints. Deliberately no progression system (no Interpol ranks).

The game was first written in Python (`carmen.py`, removed when it moved to Rust; see git history). The Rust port keeps the same rules, timings and data files.

## Files

- `src/game.rs`: the rules, with no drawing code. Constants (`DIFFICULTIES`, `PLACES`, timings) are at the top.
- `src/data.rs`: loads and validates the JSON files.
- `src/main.rs`: command-line options, the headquarters screen (`Hq`, `Step`) and the case screen with its text-panel modes (`Panel`).
- `src/computer.rs`: the crime computer drawing (monitor, stand, case); returns the screen area for the menu.
- `src/printer.rs`: the printer (at headquarters, and beside the crime computer for warrant results): queued text prints letter by letter onto tractor-feed paper, wrapped at 23 columns.
- `src/sound.rs`: sounds cut from recordings in `assets/sounds/`: `printer.wav` loops while a printer prints (`Sounds::printing`, called every frame from `Printer::update`), `key.wav` for each key typed, `tick.wav` for each hour the clock moves, `walking.wav` (five footsteps, `WALK_SECONDS`) on the way to a witness, `chase.wav` under the chase, `computing.wav` while the crime computer works. `read_wav` only checks 16-bit PCM, so convert other recordings with ffmpeg first.
- `assets/sounds/`: cut from recordings supplied by the owner with ffmpeg, mixed to mono 44.1 kHz 16-bit: printer.wav (0.7 s from a printer burst), key.wav (one key press), tick.wav (one clock tick), walking.wav (the whole 1.86 s recording).
- `src/ui.rs`: palette, pixel text (with fake bold and a fallback font), frames, buttons, the four icons and the mouse pointer.
- `src/scene.rs`: the city postcards' toolkit: `Painter` (draws with the time of day's lighting) and reusable parts (sky, clouds, mountains, hills, water, grass, buildings with windows, palms and other trees, people), plus a generic scene for countries without their own.
- `src/landmarks.rs`: one scene per country, matched by country name in `paint()`, each built around a landmark (pyramids, Eiffel Tower, Opera House, Angel Falls, Atomium...). A new country falls back to the generic scene until it gets a function here.
- `src/places.rs`: a 104x68 pixel-art picture for each place in `PLACES` (bank, airport, ...), matched by name; a new place needs a drawing here or it shows blank.
- `src/people.rs`: 66x96 portraits: one per witness in `PLACES`, and one per suspect for the dossiers. A suspect's photo in `assets/suspects/<photo_name>.png` wins; otherwise one is drawn (`suspect_look` reads hair colour and style, beards and glasses from the words in their traits; skin from the optional `skin` field, else from the name). Each is a `Look` (skin, hair, hat, glasses, beard, outfit) painted from one head-and-shoulders template; a new witness needs an entry in `look()` or gets a plain default.
- `src/sprites.rs`: the chase's running characters (the suspect, a bunch of three police) as pixel grids with two leg frames each, drawn at triple size and clipped to a rectangle.
- `src/pixels.rs`: the small software canvas both use (rects, discs, triangles, dithering, a 3x5 sign alphabet), turned into hard-edged textures.
- `src/map.rs`: the departure map: Natural Earth land rasterised for a view fitted around the current city and its flights.
- `src/rng.rs`: a small seedable RNG (SplitMix64), so `--seed` replays a case without the `rand` crate.
- `web/`: `index.html` and `mq_js_bundle.js` (macroquad's JS loader, copied from the macroquad 0.4.16 crate; recopy it from `~/.cargo/registry/src/*/macroquad-<version>/js/` when upgrading macroquad).
- `build-web.sh`: builds the browser version into `dist/` (git-ignored).
- `suspects.json`: suspects (the owner's family and friends), trait definitions and witness sentence templates.
- `countries.json`: 32 countries with about 215 clues, grouped by type (flag, currency, language, landmark, geography, culture), plus city coordinates, an `about` line and stealable treasures.
- `assets/ne_110m_land.geojson`: Natural Earth 1:110m land outlines (public domain), drawn as the world map.
- `assets/fonts/`: Pixelify Sans (the pixel font), VT323 (the printer's monospaced font, crisp at 20 px) and Noto Sans Regular (only for ₹, which Pixelify lacks; see `MISSING` in `ui.rs`). All SIL OFL; licences alongside.
- `assets/suspects/`: dossier photos, 66x96 PNGs named like `jon-orlo.png` (`people::photo_name`). Made from real photos with `tools/portrait.sh PHOTO "Name" [CROP]` (ImageMagick: crop, shrink, 28 colours, no dithering). `build.rs` embeds whatever is there, so add a photo and rebuild. The source photos themselves are not kept in the repo.
- `build.rs`: generates `photos.rs` (the embedded photo list) into `OUT_DIR`.
- `tools/portrait.sh`: turns a photo into a dossier portrait.
- `.cargo/config.toml`: lets the wasm build leave quad-snd's browser audio functions undefined; `mq_js_bundle.js` supplies them at load time. Without it the web build fails to link.
- `README.md`: player and editor docs.

## Commands

- `cargo run`: play
- `cargo run -- --check`: validate the data files. Run this after any data edit.
- `cargo run -- --seed N --difficulty 1|2|3`: reproducible case (Enter on the title screen starts that difficulty)
- `cargo test`: data validity, rules and a 300-seed-per-difficulty run of a careful player
- `cargo build --release`: single binary in `target/release/`
- `./build-web.sh`: browser build in `dist/` (needs `rustup target add wasm32-unknown-unknown`); serve it with `python3 -m http.server -d dist 8000`

## Web build

The JS console shows `register_plugin is not defined` from the bundle's networking plugin; it's a bug in that bundled plugin, harmless here. Browsers keep audio silent until the first key press or click.

macroquad compiles to `wasm32-unknown-unknown` and runs in a canvas through its own JS loader (no wasm-bindgen). Things that differ in the browser: `std::time::SystemTime` panics (use `miniquad::date::now()`), there is no filesystem (the embedded data is used) and no command-line options, and there is no Quit button (`CAN_QUIT`).

## Data loading

`--data DIR` if given; otherwise the current folder if it has `countries.json`; otherwise the copies compiled into the binary with `include_str!`. So a release binary can be handed out on its own, but it keeps whatever data it was built with: rebuild after editing the JSON.

## Architecture

- `Game` holds all rules: `investigate()`, `travel()`, `compute_warrant()`, `destinations()`, `give_up()`. It never draws or prints. Everything the player should read is pushed to `game.log` as `Line { tone, speaker, text }`; the front end styles each `Tone`.
- A case is generated in `Game::new`: the thief, a route of `legs + 1` countries, the treasure, a trait-reveal plan and the hideout place. Witness statements, places and decoy flights are generated lazily on first visit and cached, so the same seed plus the same actions replays identically. The GUI asks for places and destinations once on arrival in each city (`Case::refresh`).
- Trail city `i` has witnesses giving three distinct clues about `route[i+1]` (each place prefers certain clue types, see `PLACES`). Some witnesses add a trait sentence according to `trait_plan`. Wrong cities say "haven't seen anyone". In the final city, one of the three places is the hideout (the arrest); the other two give warnings plus a trait.
- `needed_traits()` greedily finds the traits that single out the thief. `plan_traits()` makes sure every trait is revealed along the trail, then repeats the needed ones.
- Flights: decoys exclude all route countries. Wrong cities always offer a flight back to the last trail city. Destinations are sorted by city name so the order gives nothing away.
- Time: the clock starts Monday 09:00. Interviews cost 1, 2 then 3 hours per city, flight time is distance / 800 km/h + 1 hour (rounded half-to-even, as Python's `round` did), and sleep runs 23:00 to 07:00. The deadline is computed per case (`compute_deadline`): a careful player's par time plus `spare_detours` average detours plus a `buffer`, set per difficulty in `DIFFICULTIES`.
- `witness_pronouns` (in suspects.json): true means witnesses use he/she/they and the warrant has a Pronoun field. False means everyone is "they". The validator warns when someone's pronoun is unique, since that gives them away.

## The window

- Everything is drawn at 640x360 into a render target, then scaled to the window with nearest-neighbour filtering: whole-number scales from 2x up, an exact fit below that (small browser windows). The render target comes out upside down, hence `flip_y` when drawing it. `Ui::begin_frame` converts the mouse into 640x360 coordinates. The system cursor is hidden and a pixel arrow is drawn instead.
- Immediate mode: every frame redraws everything; buttons return true on the frame they're clicked, and `take_click` consumes the click so one click can't press two things.
- Layout constants (`CLOCK`, `PICTURE`, `TEXT`, the button row) are at the top of `main.rs`. The text panel shows one `Panel` at a time: briefing, story (what just happened, typed out at `TYPING_SPEED`), investigate, depart, flying, notes, crime computer, dossiers, help, give up. The picture shows the city's postcard, or the map while departing or flying.
- Investigating opens the original's pop-up window (`place_picker`): three place pictures with their names below, the hovered one highlighted, visited ones grey. While it's open, `ui.enabled` is false for the screen underneath, so only the window takes the mouse and keys; clicking outside or Escape closes it.
- A menu bar runs across the top (`MENUS`, `App::menu_bar`): Game (new case, how to play, quit), Options (sound) and Dossiers (every suspect). It takes the mouse before the screen does, and while a menu is open the screen underneath has `ui.enabled` false. A dossier (`App::dossier`, drawn by `dossier_window`) is a white sheet over whatever screen is showing, like the original's: name on a tab, photo at double size in a heavy frame, bold labels with VT323 answers, the bio as "Other". The screen underneath ignores input; the arrow keys leaf through files and a click or Escape closes it.
- The crime computer (`Panel::Crime { editing }`) draws the menu on the monitor's screen in yellow capitals, the hovered row in inverse video. Clicking a field lists its values (`editing: Some(field)`); the mouse wheel over a field steps through them. COMPUTE first sets `Case::computing`: for `COMPUTE_SECONDS` (2.1 s, the length of `computing.wav`) the lights on the computer's stand flash random EGA colours and the menu ignores input; then `Action::Computed` runs the warrant and its verdict prints on the case's own printer in the picture frame, which starts each case with "READY.". With nothing entered, the verdict comes at once.
- Choosing a new place in the picker starts a walk (`Case::walking`): the picker stays open with the place highlighted while eight small footprints appear from the list up to its picture, in time with the footsteps sound; then `Action::Arrive` runs the interview (a click or key skips; places already visited skip the walk). The clock box shows `Case::shown_t`, which catches up with `game.t` an hour at a time, ticking for each, at a steady `HOUR_SECONDS` per hour.
- When an interview corners the thief (`game.showdown`), `Panel::Chase` plays first, timed to `assets/sounds/chase.wav` (17.6 s): suspect runs across 1.6-4.5 s, "There goes the suspect!" 5.7-10.6 s, police run across 14.4-17.0 s; then `Action::EndChase` shows the arrest story. A click or key skips it. The outcome animation (right or wrong warrant) is still to come.
- An interview shows `Panel::Interview`: the witness portrait bottom left with their job under it in VT323, and the statement typing out in a white speech bubble that grows upwards. Other lines from the same action (the time, a night's sleep) go small at the top. The arrest and running out of time stay as `Panel::Story`.
- Headquarters runs Name → Assignment → Briefing: the printer prints a welcome, echoes the name as it's typed, then prints the chosen case's briefing one paragraph at a time, pausing (`Printer::pause`) for a key between paragraphs because only about ten lines fit on the visible paper. Prompts go in the right panel and the white strip under it says what to press. A key or click skips the printing. Finishing a case and choosing "New case" comes back here, skipping the name.
- Sounds load in a coroutine so the first frame isn't held up (decoding is asynchronous in browsers). Key strikes are capped at one per 45 ms (`MIN_GAP`) so fast printing sounds like brisk typing. The on/off toggle is in the headquarters panel and the case notes.
- Screens return an `Action`; `App::apply` is the only place that changes game state. Flights animate first, then `game.travel()` runs on landing.
- Postcards are 274x282 and drawn pixel by pixel. Randomness is seeded from the country name, so a city always looks the same. The time of day sets the sky and lighting: draw through `Painter` methods for lit colours, and straight onto `p.cv` for light sources (lit windows, lamps, neon) that shouldn't darken at night. They're cached per (country, time of day).
- Pixelify Sans is crisp at 16 and 13 px (`BIG`, `SMALL`); other sizes blur. Bold is faked by drawing twice, one pixel apart.

## Calibration (simulated 400 cases per cell, done on the Python version)

A player who interviews all three witnesses in every trail city can always narrow the warrant to one person. With 2 interviews per city it works about 98-100% of the time; with 1, about 65-85%. Win rate for that full-interview player, by wrong turns taken:

| Difficulty | 0 wrong | 1 wrong | 2 wrong |
|---|---|---|---|
| Rookie | 100% | ~100% | ~80% |
| Detective | 100% | ~80% | ~27% |
| Super Sleuth | 100% | ~12% | ~1% |

The 0-wrong column is covered by `a_careful_player_on_the_right_trail_always_wins`. If you change timing constants, re-run a similar simulation by driving `Game` headless in a test.

## Content rules for clues

- Never name the country or the arrival city (`--check` warns on this).
- Avoid facts that go stale: leaders, populations, records.
- Use `{They}/{they}/{Them}/{them}/{Their}/{their}` and verbs that work for he, she and they. Simple past tense is safest; avoid was/were and is/are.
- Watch ambiguity across countries. Kenya, Uganda and Tanzania all use shillings; dollars, pesos and euros are shared; Spanish, Swahili and Portuguese span several countries. Those currency and language clues are left out where they'd be ambiguous.
- Clue facts were written from model knowledge and not individually source-checked. Spot-check them before relying on them.
- British spelling throughout (colour, favourite, practised).

## Ideas not yet done

- Sound: macroquad's `audio` feature is off; turn it on in Cargo.toml and play effects on flights, interviews and the arrest.
- Art: hand-drawn city pictures to replace the generated postcards, witness portraits
- Two-player mode on the same seed (fastest time wins)
- More witness places (`PLACES`), more countries
