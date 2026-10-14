# Where in the World?

A remake of Brøderbund's 1990 *Where in the World is Carmen Sandiego?*, in Rust. The thieves are your family and friends, and the facts are up to date. It runs on desktops and in the browser, including on phones, in English or Spanish (Argentina).

Play it at **https://jonathan-orlowski.dev/carmen**.

![A case from start to finish: the briefing at headquarters, a flight, a henchman, the crime computer, a warning in the final city, the chase and the jail](docs/demo.gif)

The whole game is a 1.3 MB download: small enough for a 1.44 MB floppy disk, if you still have one.

## Playing

Someone in the family has stolen a treasure. Follow the trail around the world, get a warrant from the crime computer and make the arrest before the deadline.

- **Investigate** (the magnifying glass): each city has three places, each with a witness. On the right trail, witnesses hint at the next country (flags, money, landmarks, food, language) and sometimes describe the thief. In a wrong city nobody has seen a thing, so fly back.
- **Depart** (the plane): four flights leave every city. Pick one from the list or click its dot on the map.
- **Crime computer**: enter what you've learned. When only one person fits, you get a warrant.
- **Notes** (the notebook): everything witnesses have told you, the deadline and your warrant.

S.U.R.M.E.'s henchmen show up when you're on the right track, and the final city gets dangerous. When witnesses say the thief is still in town, you're close: one of the three places is the hideout. With a warrant for the right person you watch the arrest. Without one, they get away.

The clock starts on Monday at 9 a.m. Each interview takes 3 hours, longer flights take longer, and you sleep from 11 p.m. to 7 a.m.

Click or press a key to skip typing and animations. The menu bar holds a new case, how to play and about (Game), sound (Options), the detective roster (Bureau), everyone's file (Dossiers) and the EN/ES language toggle. The game starts in your system's or browser's language and remembers the one you choose.

On a phone held upright, the picture sits above the text panel and buttons, and a text box pops up for typing your name. Turned sideways, it's the original layout.

### Ranks

As in the original, you start as a Rookie and are promoted as you make arrests. Each rank's cases are harder, and each promotion takes more arrests than the last. After every arrest, the printer tells you how many more you need.

| Rank | Arrests | Flights to the hideout | Time allowance |
|---|---|---|---|
| Rookie | 0 | 4 | about 2 wrong turns |
| Sleuth | 1 | 4 | about 2 wrong turns, less spare |
| Private Eye | 3 | 5 | about 1 wrong turn |
| Investigator | 6 | 5 | about 1 wrong turn, less spare |
| Ace Detective | 10 | 6 | about 1 wrong turn |
| Super Sleuth | 15 | 6 | almost none |

Your arrests are saved under your name, which can be up to 8 characters. There are no passwords: it's for family and friends. They're kept on the bureau's server, so your rank follows you to any device, and on the device itself, so the game carries on offline and catches up later: in the browser's storage, or on a computer in `~/.local/share/where-in-the-world/` (Linux), `%APPDATA%\where-in-the-world\` (Windows) or `~/Library/Application Support/where-in-the-world/` (macOS). Bureau → Detective roster lists everyone.

## Running it yourself

You need Rust (https://rustup.rs). In this folder:

```
cargo run
```

Options go after `--`, for example `cargo run -- --seed 42`:

- `--seed 42` replays the same case, handy for challenging someone to beat your time.
- `--rank 1` to `--rank 6` plays cases as hard as that rank, whatever your real one. Arrests still count.
- `--check` checks your edited data files and points at any mistakes.
- `--data FOLDER` reads the data files from another folder.

`cargo build --release` makes a single program in `target/release/` with the data built in, so rebuild after editing the data.

### On the web

`./build-web.sh` builds the browser version into `dist/` (the first time, also run `rustup target add wasm32-unknown-unknown`). Try it with `python3 -m http.server -d dist 8000`. The eight files in `dist/` work on any static web host.

Pushing to `main` deploys it to Cloudflare Pages through GitHub Actions (`.github/workflows/deploy-web.yml`). That needs the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets and a Pages project called `carmen`.

### The server

`server/` keeps each detective's arrests in Postgres. GitHub Actions builds it into the image `ghcr.io/jonorl/carmen-api` and deploys it to the VPS (`.github/workflows/deploy-server.yml`) when `server/` or `shared/` change. It reads three settings:

- `CARMEN_DB_URL`: the Postgres database.
- `PORT_CARMEN`: the port to listen on (default 8085).
- `FRONTEND_URL_CARMEN`: the game's addresses, separated by commas, which browsers may call it from.

It creates its table itself. The workflow needs the `GHCR_TOKEN` (with `write:packages`), `VPS_HOST`, `VPS_USER` and `VPS_SSH_KEY` secrets: the last is a private key whose public half is in the VPS user's `~/.ssh/authorized_keys`.

The game calls `https://carmen-api.jonathan-orlowski.dev`. Build it with `CARMEN_API` set to use another address, or set to nothing to keep progress on the device only. To try it all locally:

1. Run Postgres.
2. Start the server with `CARMEN_DB_URL=postgres://... FRONTEND_URL_CARMEN=http://localhost:8000 cargo run -p carmen-server`.
3. Build the game with `CARMEN_API=http://localhost:8085 ./build-web.sh`.

## Working on the code

`src/` has `main.rs` (start-up, layout and the game loop), `lang.rs` (the two languages) and `ui.rs` (fonts, frames, buttons) at the top, and four folders:

- `rules/`: the game itself, with no drawing: the rules, the data files and the random numbers. The tests drive it.
- `screens/`: what's on screen and what you click: headquarters, a case and its text panel, the windows over them and the menu bar.
- `art/`: pictures made in code or traced from the original: postcards, places, portraits, sprites, the jail and the map.
- `device/`: sound, saving, the server and the phone keyboard.

`CLAUDE.md` describes every file and how the pieces fit together. Day to day:

- `cargo test --workspace` runs the tests, including hundreds of simulated cases in both languages.
- `cargo clippy --workspace --all-targets` should stay free of warnings.
- `cargo fmt --all` formats the code at 140 columns (`rustfmt.toml`).

Pushes to `main` only deploy if the tests and `--check` pass.

The fonts are cut down to the characters the game shows, to keep the download small. If a new character shows up blank (a name in another alphabet, a new currency sign), add its range to `tools/subset-fonts.sh`, put the full font back in `assets/fonts/` and run the script.

## Putting your people in: `suspects.json`

Each person needs:

- `name`
- `sex`: `male` or `female`. Witnesses say he or she, and the crime computer has a Sex field, as in the original.
- a value for every trait (`hobby`, `hair`, `feature`, `vehicle`, `other`)
- optionally a `catchphrase`, which they say when you catch them, and a `bio`, shown as Notes in their dossier
- optionally a photo for their dossier: run `tools/portrait.sh photo.jpg "Their Name"` (it needs ImageMagick) and rebuild. It becomes a small pixel-art picture in `assets/suspects/`. If the face isn't in the middle, add the part to use as a third argument, like `300x436+82+24` (width x height + left + top).
- otherwise, optionally a `skin` tone for a drawn portrait: `light`, `rosy`, `olive`, `tan`, `brown` or `dark`. Hair, beards and glasses come from the words in their traits.

To keep it fair:

- No two people can share their sex and every crime computer trait. `--check` catches this.
- Values work best when several people share each one, like two with black hair, so the detective needs several clues.
- If someone is the only man or the only woman, the first witness gives them away. `--check` warns you.
- 8 to 12 people keeps cases interesting.

### Traits

You can rename, add or remove traits in the `traits` list. Each has:

- `key`: the field name used on each person
- `label`: shown in the crime computer and the dossiers
- `clues`: witness sentences, where `{value}` becomes the person's value, and `{They}`/`{they}`, `{Them}`/`{them}` and `{Their}`/`{their}` become he/him/his or she/her/her
- optionally `phrases`: how witnesses word a value. With `"bald": "no hair at all"`, the computer shows Bald and a witness says "He had no hair at all."
- optionally `"computer": false`: witnesses mention it and the dossier shows it, but it can't go on a warrant. `other` works this way; write it to follow "he" or "she", like "collects stamps".

Inside jokes make the best traits.

### In Spanish

Each trait has an `es` section with four parts:

- `label`
- `clues`: its witness sentences
- `values`: what the crime computer shows, like `"bald": "pelado"`
- `phrases`: what witnesses say, like `"bald": "la cabeza pelada"`

Each person's `es` section has their own Spanish for a trait. `other` goes there, written for their sex: "se hace el que sabe japonés". Where a word agrees with the thief, Spanish sentences use `{Él}`/`{él}`, `{lo}` (lo or la) and `{o}` (cansad{o}). Catchphrases stay in whatever language the person says them.

## Countries and clues: `countries.json`

There are 33 countries with about 220 clues. They avoid facts that go stale, such as leaders, populations and records. Each country has:

- `country` and `city` (where you land), and `lat`/`lon` for the city, which set flight times
- `about`: a line shown on arrival
- `treasures`: things that can be stolen there
- `clues` grouped by type: `flag`, `currency`, `language`, `landmark`, `geography` and `culture`. Witnesses favour certain types: the bank teller likes currency, the check-in agent likes flags.
- `es`: all of the above in Spanish, with the treasures in the same order and the same number of clues of each type. Spanish clues usually leave out the subject ("Preguntó por una bandera...").

A clue should point to its country without naming it or the arrival city. Clues also have to work against each other: Kenya, Uganda and Tanzania all use shillings, so currency clues are left out where they'd be ambiguous. `--check` warns about clues that name the answer, and about Spanish that doesn't match the English.

## Credits

- The original game: *Where in the World is Carmen Sandiego?* © 1990 Brøderbund Software. This is a fan remake, made for fun. The animations, jail and sounds were traced and cut from recordings of the original.
- Map: Natural Earth (public domain).
- Fonts: Pixelify Sans, VT323 and Noto Sans, under the SIL Open Font License (licences in `assets/fonts/`).

## Licence

The code is under the MIT licence (`LICENSE`). That doesn't cover the original game's art and sounds traced into `assets/sprites/` and `assets/sounds/`, which belong to their owners, the photos of real people in `assets/suspects/`, or the fonts, which keep their own licences.
