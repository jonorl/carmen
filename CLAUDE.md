# Where in the World? (family edition)

A CLI remake of the 1990s DOS game "Where in the World is Carmen Sandiego?". The thieves are the player's family and friends instead of V.I.L.E. henchmen, and the country facts are updated. CLI only for now: no graphics, no music, and deliberately no progression system (no Interpol ranks).

## Files

- `carmen.py`: the whole game, Python 3.8+ standard library only. Keep it dependency-free.
- `suspects.json`: suspects (placeholders for now; the owner will swap in real family and friends), trait definitions and witness sentence templates.
- `countries.json`: 48 countries with about 300 clues, grouped by type (flag, currency, language, landmark, geography, culture), plus city coordinates, an `about` line and stealable treasures.
- `README.md`: player and editor docs.

## Commands

- `python3 carmen.py`: play
- `python3 carmen.py --check`: validate the data files. Run this after any data edit.
- `python3 carmen.py --seed N --difficulty 1|2|3`: reproducible case

## Architecture

- The `Game` class holds all logic: `investigate()`, `travel()`, `compute_warrant()`, `destinations()`. The `ui_*` methods are the terminal menus. Output goes through `self.out`, so the logic can run headless (pass `out=lambda *a, **k: None`) for simulations or a future GUI.
- A case is generated in `Game.__init__`: the thief, a route of `legs + 1` countries, the treasure, a trait-reveal plan and the hideout place. Witness statements, places and decoy flights are generated lazily on first visit and cached, so the same `--seed` plus the same actions replays identically.
- Trail city `i` has witnesses giving three distinct clues about `route[i+1]` (each place prefers certain clue types, see `PLACES`). Some witnesses add a trait sentence according to `trait_plan`. Wrong cities say "haven't seen anyone". In the final city, one of the three places is the hideout (the arrest); the other two give warnings plus a trait.
- `_needed_traits()` greedily finds the traits that single out the thief. `_plan_traits()` makes sure every trait is revealed along the trail, then repeats the needed ones.
- Flights: decoys exclude all route countries. Wrong cities always offer a flight back to the last trail city. Destinations are sorted by city name so the order gives nothing away.
- Time: the clock starts Monday 09:00. Interviews cost 1, 2 then 3 hours per city, flight time is distance / 800 km/h + 1 hour, and sleep runs 23:00 to 07:00. The deadline is computed per case (`_deadline`): a careful player's par time plus `spare_detours` average detours plus a `buffer`, set per difficulty in `DIFFICULTIES`.
- `witness_pronouns` (in suspects.json): true means witnesses use he/she/they and the warrant has a Pronoun field. False means everyone is "they". The validator warns when someone's pronoun is unique, since that gives them away.

## Calibration (simulated 400 cases per cell)

A player who interviews all three witnesses in every trail city can always narrow the warrant to one person. With 2 interviews per city it works about 98-100% of the time; with 1, about 65-85%. Win rate for that full-interview player, by wrong turns taken:

| Difficulty | 0 wrong | 1 wrong | 2 wrong |
|---|---|---|---|
| Rookie | 100% | ~100% | ~80% |
| Detective | 100% | ~80% | ~27% |
| Super Sleuth | 100% | ~12% | ~1% |

If you change timing constants, re-run a similar simulation by driving `Game` headless.

## Content rules for clues

- Never name the country or the arrival city (`--check` warns on this).
- Avoid facts that go stale: leaders, populations, records.
- Use `{They}/{they}/{Them}/{them}/{Their}/{their}` and verbs that work for he, she and they. Simple past tense is safest; avoid was/were and is/are.
- Watch ambiguity across countries. Kenya, Uganda and Tanzania all use shillings; dollars and pesos are shared; Swahili and Portuguese span several countries. Those currency and language clues are left out where they'd be ambiguous.
- Clue facts were written from model knowledge and not individually source-checked. Spot-check them before relying on them.
- British spelling throughout (colour, favourite, practised).

## Ideas not yet done

- Graphics or sound front end reusing `Game`
- Two-player mode on the same seed (fastest time wins)
- More witness places (`PLACES`), more countries
- Real family and friends in suspects.json (the owner's job)
