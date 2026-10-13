# Where in the World?

A remake of the 1990s detective chase, with a world map. The thieves are your family and friends.

## Run it

You need Rust (https://rustup.rs). In this folder:

```
cargo run
```

Options go after `--`, for example `cargo run -- --seed 42`:

- `--rank 1` to `--rank 6` plays cases as hard as that rank (Rookie to Super Sleuth), whatever your real rank. Arrests still count towards promotion.
- `--seed 42` replays the same case, which is handy for challenging someone to beat your time.
- `--check` checks your edited data files and points at any mistakes.
- `--data FOLDER` reads the JSON files from another folder.

To give the game to someone, run `cargo build --release` and send them `target/release/carmen` (`carmen.exe` on Windows, built on Windows). The data files are built into it, so rebuild after editing them.

To put it on a website, run `./build-web.sh` (the first time, also `rustup target add wasm32-unknown-unknown`) and upload the six files in `dist/` to any web host. They're plain static files. Pushing to `main` does this for you: GitHub Actions builds the game and puts it on Cloudflare Pages (`.github/workflows/deploy-web.yml`, which needs the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets and a Pages project called `carmen`).

On a phone held upright, the picture moves above the text panel and buttons, and a text box pops up for typing your name. Turned sideways, it's the original layout.

The menu bar along the top has Game (new case, how to play), Options (sound) and Dossiers, where you can read anyone's file at any time, even at headquarters. The four buttons under the text are the notebook (everything witnesses told you, plus help), the plane (flights), the magnifying glass (interview witnesses) and the crime computer (warrants and dossiers). Click or press a key to skip the typing. Scroll the notes with the mouse wheel.

## How a case works

Someone in the family steals a treasure, and the clock starts Monday at 09:00. Each city has three places to investigate. On the right trail, witnesses hint at the next country and sometimes describe the thief. In a wrong city, nobody has seen anything, so fly back.

Enter what you've learned into the crime computer. When only one person fits, you get a warrant. In the hideout city, witnesses warn you the thief is still in town, and one of the three places is where you catch them. Without a warrant for the right person, they get away.

Each interview costs 3 hours, flights take longer the farther you go, and you sleep from 23:00 to 07:00. The deadline is worked out for each case from the route, so a far-flung trail gets more time.

## Ranks

As in the original, you start as a Rookie and get promoted as you make arrests. Each rank's cases are harder, and each promotion takes more arrests than the last. After every arrest the printer says how many more you need. Your arrests are saved under your name, which can be up to 8 characters (there are no passwords: it's for family and friends). They're kept on the agency's server, so your rank follows you to any computer or phone, and also on the device itself: in a file on your computer (`~/.local/share/where-in-the-world/progress.json` on Linux, the app-data folder on Windows and macOS), or in the browser's storage on the website. If the server can't be reached, the game carries on with the device's copy, and the two catch up next time. Bureau → Detective roster lists every detective with their rank and arrests.

## The server

`server/` keeps the arrests in Postgres. It's built into the image `ghcr.io/jonorl/carmen-api` and deployed to the VPS by `.github/workflows/deploy-server.yml` when `server/` or `shared/` change. It reads `CARMEN_DB_URL`, `PORT_CARMEN` (default 8085) and `FRONTEND_URL_CARMEN` (the game's addresses, separated by commas, which browsers may call it from), and creates its table itself on start. The game calls `https://carmen-api.jonathan-orlowski.dev` unless it's built with `CARMEN_API` set to another address, or to nothing to keep progress on the device only. To try it locally, run Postgres, then `CARMEN_DB_URL=postgres://... FRONTEND_URL_CARMEN=http://localhost:8000 cargo run -p carmen-server`, and build the game with `CARMEN_API=http://localhost:8085 ./build-web.sh`.

| Rank | Arrests | Flights to the hideout | Time allowance |
|---|---|---|---|
| Rookie | 0 | 4 | about 2 wrong turns |
| Sleuth | 1 | 4 | about 2 wrong turns, less spare |
| Private Eye | 3 | 5 | about 1 wrong turn |
| Investigator | 6 | 5 | about 1 wrong turn, less spare |
| Ace Detective | 10 | 6 | about 1 wrong turn |
| Super Sleuth | 15 | 6 | almost none |

## Putting your people in: `suspects.json`

Replace the placeholder people. Each person needs:

- `name`
- `sex`: `male` or `female`. Witnesses say he or she, and the crime computer has a Sex field, as in the original game.
- a value for every trait (`hobby`, `hair`, `feature`, `vehicle`, `other`)
- optionally a `catchphrase` (what they say when you catch them) and a `bio` (shown as Notes in their dossier)
- optionally a real photo for their dossier: run `tools/portrait.sh photo.jpg "Their Name"` (it needs ImageMagick) and rebuild. It's turned into a small pixel-art picture in `assets/suspects/`. If the face isn't in the middle, add the part to use as a third argument, like `300x436+82+24` (width x height + left + top, in the photo's pixels). The pictures are built into the game, so anyone you give it to, or who plays it on your website, sees them.
- otherwise, optionally a `skin` tone for their drawn dossier picture: `light`, `rosy`, `olive`, `tan`, `brown` or `dark`. Without it, one is picked from their name. The rest of the photo comes from the words in their traits: "bald", "wavy brown hair", "beard" or "glasses" all show up.

A few rules keep the game fair:

- No two people can have the same sex and the same crime computer traits. `--check` catches this.
- Trait values work best when several people share each one, like two with black hair or two who get around by bicycle. The detective then needs several clues to narrow it down.
- If someone is the only man or the only woman, the first witness gives them away; `--check` warns you.
- With 8 to 12 people, cases stay interesting.

### Changing the traits

You can rename, add or remove traits in the `traits` list. Each trait has:

- `key`: the field name used on each person
- `label`: shown in the crime computer and the dossiers
- `clues`: witness sentences, where `{value}` becomes the person's value
- optionally `phrases`: how witnesses word a value that doesn't read well on its own. With `"phrases": {"bald": "no hair at all"}`, the computer shows Bald and a witness says "He had no hair at all."
- optionally `"computer": false`: the crime computer doesn't have this trait, so it can't go on a warrant, but witnesses still mention it and the dossier shows it. `other` works this way: a one-off detail that helps you guess who it is. Write it to follow "he" or "she", like "collects stamps".

Use `{They}`/`{they}`, `{Them}`/`{them}` and `{Their}`/`{their}` for the thief; they become he/him/his or she/her/her.

Inside jokes make the best traits: "Hobby: arguing about football", "Other: falls asleep during films".

## Countries and clues: `countries.json`

There are 33 countries with about 220 clues, written to stay accurate. They avoid leaders, populations and records that change, and they include newer facts like Türkiye's 2022 name change.

Each country has:

- `country` and `city` (where you land)
- `lat`/`lon` for the city, which sets flight times
- `about`: a line shown on arrival
- `treasures`: things that can be stolen there
- `clues` grouped by type: `flag`, `currency`, `language`, `landmark`, `geography` and `culture`

Witnesses favour certain types. The bank teller likes currency, the check-in agent likes flags and the librarian likes language. A clue should point to the country without naming it or the arrival city, and `--check` warns if one does.

Clues also have to work against each other. Kenya, Uganda and Tanzania all use shillings, and several countries use dollars or pesos, so currency clues are left out where they'd be ambiguous. Keep that in mind when adding countries.

## Ideas for later

- More sound: flights, interviews, the arrest, music.
- Two-player mode: same `--seed`, fastest time wins.
- More witness places: add them to the `PLACES` list near the top of `src/game.rs`.

## Credits

- Map: Natural Earth (public domain).
- Fonts: Pixelify Sans, VT323 and Noto Sans, all under the SIL Open Font License (licences in `assets/fonts/`).
- Sound: recordings you supplied (`assets/sounds/`).
