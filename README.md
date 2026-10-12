# Where in the World?

A terminal remake of the 1990s detective chase. The thieves are your family and friends.

## Run it

Put the three files in one folder and run:

```
python3 carmen.py
```

On Windows, type `python` instead of `python3`. It needs Python 3.8 or newer and no extra packages.

Options:

- `--difficulty 1|2|3` skips the difficulty menu (Rookie, Detective, Super Sleuth).
- `--seed 42` replays the same case, which is handy for challenging someone to beat your time.
- `--check` checks your edited data files and points at any mistakes.

## How a case works

Someone in the family steals a treasure, and the clock starts Monday at 09:00. Each city has three places to investigate. On the right trail, witnesses hint at the next country and sometimes describe the thief. In a wrong city, nobody has seen anything, so fly back.

Enter what you've learned into the crime computer. When only one person fits, you get a warrant. In the hideout city, witnesses warn you the thief is still in town, and one of the three places is where you catch them. Without a warrant for the right person, they get away.

Each new interview in a city costs more time (1, 2, then 3 hours), flights take longer the farther you go, and you sleep from 23:00 to 07:00. The deadline is worked out for each case from the route, so a far-flung trail gets more time.

| Difficulty | Flights to the hideout | Time allowance |
|---|---|---|
| Rookie | 4 | about 2 wrong turns |
| Detective | 5 | about 1 wrong turn |
| Super Sleuth | 6 | almost none |

## Putting your people in: `suspects.json`

Replace the ten placeholder people. Each person needs:

- `name`
- `pronoun`: `he`, `she` or `they`
- a value for every trait (`hair`, `hobby`, `food`, `vehicle`, `quirk`)
- optionally a `bio` (shown in the dossiers) and a `catchphrase` (what they say when you catch them)

A few rules keep the game fair:

- No two people can have the same pronoun and identical traits. `--check` catches this.
- Trait values work best when several people share each one, like two with red hair or three who love karaoke. The detective then needs several clues to narrow it down.
- With 8 to 12 people and 5 traits, cases stay interesting.

### Pronouns

With `"witness_pronouns": true`, witnesses say he, she or they, and the crime computer has a Pronoun field, like the original game. If someone is the only "they" or the only "she", the very first witness gives them away. In that case, set `"witness_pronouns": false` and every witness says "they". `--check` warns you when this applies.

### Changing the traits

You can rename, add or remove traits in the `traits` list. Each trait has:

- `key`: the field name used on each person
- `label`: shown in the crime computer
- `clues`: witness sentences, where `{value}` becomes the person's value

Use `{They}`/`{they}`, `{Them}`/`{them}` and `{Their}`/`{their}` for the thief. Stick to verbs that work for he, she and they. Simple past tense is safest ("had", "kept", "ordered"); avoid "was/were" and "is/are".

Inside jokes make the best traits: "Quirk: falling asleep during films", "Hobby: arguing about football", "Food: Mum's pilau".

## Countries and clues: `countries.json`

There are 48 countries with about 300 clues, written to stay accurate. They avoid leaders, populations and records that change, and they include newer facts like Türkiye's 2022 name change, Croatia's switch to the euro and Astana's two name changes.

Each country has:

- `country` and `city` (where you land)
- `lat`/`lon` for the city, which sets flight times
- `about`: a line shown on arrival
- `treasures`: things that can be stolen there
- `clues` grouped by type: `flag`, `currency`, `language`, `landmark`, `geography` and `culture`

Witnesses favour certain types. The bank teller likes currency, the check-in agent likes flags and the librarian likes language. A clue should point to the country without naming it or the arrival city, and `--check` warns if one does.

Clues also have to work against each other. Kenya, Uganda and Tanzania all use shillings, and several countries use dollars or pesos, so currency clues are left out where they'd be ambiguous. Keep that in mind when adding countries.

## Ideas for later

- Graphics or sound: the game logic lives in the `Game` class, separate from the menus, so a GUI or web front end could reuse it.
- Two-player mode: same `--seed`, fastest time wins.
- More witness places: add them to the `PLACES` list near the top of `carmen.py`.
