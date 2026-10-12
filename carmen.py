#!/usr/bin/env python3
"""
WHERE IN THE WORLD?  A family-and-friends remake of the classic 1990s
detective chase, played in the terminal.

    python3 carmen.py              play
    python3 carmen.py --check      check your edited data files for mistakes
    python3 carmen.py --seed 42    replay exactly the same case

Needs Python 3.8 or newer and nothing else.
Your suspects (family and friends) live in suspects.json.
Countries, cities and clues live in countries.json.
"""

import argparse
import json
import math
import os
import random
import re
import sys
import textwrap

# ------------------------------------------------------------------ settings

GAME_TITLE = "WHERE IN THE WORLD?"
AGENCY = "Family Bureau of Investigation"

START_HOUR = 9                  # every case starts Monday 09:00
BEDTIME, WAKE_UP = 23, 7        # detectives sleep from 23:00 to 07:00
INTERVIEW_HOURS = (1, 2, 3)     # 1st, 2nd and 3rd interview in the same city
WARRANT_HOURS = 1               # each run of the crime computer
FLIGHT_KMH = 800                # speed used to turn distance into flight time
AIRPORT_HOURS = 1               # check-in, taxis and so on, added to each flight

DIFFICULTIES = (
    # legs: flights from the crime scene to the hideout
    # spare_detours: roughly how many wrong turns you can afford
    # buffer: extra hours on top of that
    {"name": "Rookie", "legs": 4, "spare_detours": 2, "buffer": 12},
    {"name": "Detective", "legs": 5, "spare_detours": 1, "buffer": 6},
    {"name": "Super Sleuth", "legs": 6, "spare_detours": 0, "buffer": 2},
)

# Places you can investigate: (place, witness, clue types that witness favours)
PLACES = (
    ("Bank", "Bank teller", ("currency",)),
    ("Airport", "Check-in agent", ("flag", "geography")),
    ("Library", "Librarian", ("language", "landmark")),
    ("Museum", "Museum guide", ("landmark", "culture")),
    ("Market", "Market trader", ("culture", "currency")),
    ("Hotel", "Hotel concierge", ()),
    ("Bus station", "Ticket seller", ("geography", "landmark")),
    ("Café", "Barista", ("culture", "language")),
    ("Taxi rank", "Taxi driver", ("landmark", "geography")),
)
CLUE_TYPES = ("flag", "currency", "language", "landmark", "geography", "culture")

# What witnesses say when the thief never came through their city.
NOBODY = (
    "Sorry, I haven't seen anyone like that around here.",
    "Nope. Nobody like that came through today.",
    "I'd remember a face like that. Haven't seen one.",
    "Never seen anyone matching that description, sorry.",
    "Quiet day. Nobody unusual at all.",
    "Can't help you, detective. Nobody like that.",
)

# What witnesses say in the hideout city.
WARNINGS = (
    "Careful, detective! Someone like that rushed past here not an hour ago. "
    "{They} must still be in town!",
    "{They} asked me about good places to hide around here. Watch your step!",
    "I saw {them} a moment ago, looking over {their} shoulder. "
    "{They} can't have gone far!",
    "{They} said {they} planned to lie low in this city for a while. Hurry!",
)

SPOTTED = (
    "you spot {name} trying to slip out of the back door!",
    "you spot {name} hiding behind a newspaper with two eye-holes cut in it!",
    "you spot {name} tiptoeing away with a suspiciously heavy bag!",
)

PRONOUNS = {
    "he": ("he", "him", "his"),
    "she": ("she", "her", "her"),
    "they": ("they", "them", "their"),
}
PRONOUN_KEYS = {"They", "they", "Them", "them", "Their", "their"}
PLACEHOLDER = re.compile(r"\{(\w+)\}")
RESERVED_KEYS = {"name", "pronoun", "bio", "catchphrase"}
DAYS = ("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")
WIDTH = 72

# ------------------------------------------------------------ text helpers


class Colour:
    on = False


def _paint(code, text):
    return f"\033[{code}m{text}\033[0m" if Colour.on else text


def bold(t): return _paint("1", t)
def dim(t): return _paint("2", t)
def red(t): return _paint("1;31", t)
def green(t): return _paint("1;32", t)
def yellow(t): return _paint("1;33", t)
def cyan(t): return _paint("36", t)


def wrap(text, indent="", hang=None):
    return textwrap.fill(text, WIDTH, initial_indent=indent,
                         subsequent_indent=indent if hang is None else hang)


def fill(template, pronoun, value=None):
    """Swap {They}/{them}/{their}/{value} placeholders for real words."""
    they, them, their = PRONOUNS[pronoun]
    subs = {"they": they, "them": them, "their": their,
            "They": they.capitalize(), "Them": them.capitalize(),
            "Their": their.capitalize()}
    if value is not None:
        subs["value"] = value
    text = PLACEHOLDER.sub(lambda m: subs.get(m.group(1), m.group(0)), template)
    return text[:1].upper() + text[1:]


def sentence_case(text):
    return text[:1].upper() + text[1:]


def clock_text(t):
    day, hour = divmod(START_HOUR + t, 24)
    name = DAYS[day % 7]
    if day >= 7:
        name = "next " + name
    return f"{name} {hour:02d}:00"


def duration_text(hours):
    days, hrs = divmod(hours, 24)
    parts = []
    if days:
        parts.append(f"{days} day{'s' if days != 1 else ''}")
    if hrs or not days:
        parts.append(f"{hrs} hour{'s' if hrs != 1 else ''}")
    return ", ".join(parts)


def after_sleep(t):
    """If it's night at time t, return the time you wake up; otherwise t."""
    hour = (START_HOUR + t) % 24
    if hour >= BEDTIME:
        return t + (24 - hour) + WAKE_UP
    if hour < WAKE_UP:
        return t + (WAKE_UP - hour)
    return t


def km_between(a, b):
    lat1, lon1, lat2, lon2 = map(math.radians, (a["lat"], a["lon"], b["lat"], b["lon"]))
    h = (math.sin((lat2 - lat1) / 2) ** 2
         + math.cos(lat1) * math.cos(lat2) * math.sin((lon2 - lon1) / 2) ** 2)
    return 2 * 6371 * math.asin(math.sqrt(h))


def flight_hours(a, b):
    return max(2, round(km_between(a, b) / FLIGHT_KMH) + AIRPORT_HOURS)


def place_label(c):
    return f"{c['city']}, {c['country']}"


def same(a, b):
    return a.strip().lower() == b.strip().lower()

# ------------------------------------------------------------- data files


class DataError(Exception):
    pass


def read_json(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        raise DataError(f"Can't find {path}")
    except json.JSONDecodeError as e:
        raise DataError(
            f"{os.path.basename(path)}, line {e.lineno}, column {e.colno}: {e.msg}.\n"
            "  (Usually a missing comma, quote or bracket just before that spot.)")


def validate(cdoc, sdoc):
    """Return (errors, warnings) for the two data files."""
    errors, warnings = [], []

    # ---- countries.json
    countries = cdoc.get("countries") if isinstance(cdoc, dict) else None
    if not isinstance(countries, list):
        errors.append('countries.json needs a "countries" list.')
        countries = []
    seen = set()
    for n, c in enumerate(countries, 1):
        where = f"countries.json, entry {n}"
        if not isinstance(c, dict):
            errors.append(f"{where}: should be an object {{ ... }}.")
            continue
        if isinstance(c.get("country"), str) and c["country"].strip():
            where = f"countries.json, {c['country']}"
        for key in ("country", "city"):
            if not isinstance(c.get(key), str) or not c[key].strip():
                errors.append(f'{where}: missing "{key}".')
        for key, lo, hi in (("lat", -90, 90), ("lon", -180, 180)):
            v = c.get(key)
            if isinstance(v, bool) or not isinstance(v, (int, float)) or not lo <= v <= hi:
                errors.append(f'{where}: "{key}" should be a number from {lo} to {hi}.')
        if c.get("country") in seen:
            errors.append(f"{where}: listed twice.")
        seen.add(c.get("country"))
        t = c.get("treasures")
        if not isinstance(t, list) or not t or not all(isinstance(x, str) and x.strip() for x in t):
            errors.append(f'{where}: needs a "treasures" list with at least one item.')
        clues = c.get("clues")
        if not isinstance(clues, dict):
            errors.append(f'{where}: needs a "clues" section.')
            continue
        total = 0
        for cat, texts in clues.items():
            if not isinstance(texts, list) or not all(isinstance(x, str) and x.strip() for x in texts):
                errors.append(f'{where}: clues "{cat}" should be a list of sentences.')
                continue
            if cat not in CLUE_TYPES:
                warnings.append(f'{where}: clue type "{cat}" is not one of '
                                f'{", ".join(CLUE_TYPES)}. It still works, '
                                "but no witness prefers it.")
            for text in texts:
                total += 1
                bad = set(PLACEHOLDER.findall(text)) - PRONOUN_KEYS
                if bad:
                    errors.append(f"{where}: unknown placeholder {{{sorted(bad)[0]}}} in: {text}")
                for giveaway in (c.get("country"), c.get("city")):
                    if isinstance(giveaway, str) and giveaway and re.search(
                            r"\b" + re.escape(giveaway.lower()) + r"\b", text.lower()):
                        warnings.append(f"{where}: this clue names the answer: {text}")
        if total < 3:
            errors.append(f"{where}: needs at least 3 clues (has {total}).")
        elif total < 5:
            warnings.append(f"{where}: only {total} clues. Five or more keeps cases varied.")
    need = max(d["legs"] for d in DIFFICULTIES) + 6
    if len(countries) < need:
        errors.append(f"countries.json has {len(countries)} countries; the game needs at least {need}.")

    # ---- suspects.json
    traits = sdoc.get("traits") if isinstance(sdoc, dict) else None
    if not isinstance(traits, list) or not traits:
        errors.append('suspects.json needs a "traits" list.')
        traits = []
    keys = []
    for n, t in enumerate(traits, 1):
        if not isinstance(t, dict) or not isinstance(t.get("key"), str) or not t["key"].strip():
            errors.append(f'suspects.json, trait {n}: needs a "key".')
            continue
        k = t["key"]
        where = f'suspects.json, trait "{k}"'
        if k in keys or k in RESERVED_KEYS:
            errors.append(f"{where}: that key is used twice or is reserved "
                          f"({', '.join(sorted(RESERVED_KEYS))}).")
        keys.append(k)
        if not isinstance(t.get("label"), str) or not t["label"].strip():
            errors.append(f'{where}: needs a "label" (shown in the crime computer).')
        templates = t.get("clues")
        if not isinstance(templates, list) or not templates or not all(isinstance(x, str) for x in templates):
            errors.append(f'{where}: needs a "clues" list of witness sentences.')
            continue
        for text in templates:
            bad = set(PLACEHOLDER.findall(text)) - PRONOUN_KEYS - {"value"}
            if bad:
                errors.append(f"{where}: unknown placeholder {{{sorted(bad)[0]}}} in: {text}")
            if "{value}" not in text:
                errors.append(f"{where}: clue must contain {{value}}: {text}")

    use_pronouns = sdoc.get("witness_pronouns", True) if isinstance(sdoc, dict) else True
    if not isinstance(use_pronouns, bool):
        errors.append('suspects.json: "witness_pronouns" must be true or false.')
        use_pronouns = True
    suspects = sdoc.get("suspects") if isinstance(sdoc, dict) else None
    if not isinstance(suspects, list) or len(suspects) < 2:
        errors.append('suspects.json needs a "suspects" list with at least 2 people.')
        suspects = []
    names, profiles = set(), {}
    for n, s in enumerate(suspects, 1):
        where = f"suspects.json, suspect {n}"
        if not isinstance(s, dict):
            errors.append(f"{where}: should be an object {{ ... }}.")
            continue
        name = s.get("name")
        if not isinstance(name, str) or not name.strip():
            errors.append(f'{where}: missing "name".')
            continue
        where = f"suspects.json, {name}"
        if name in names:
            errors.append(f"{where}: name used twice.")
        names.add(name)
        if s.get("pronoun") not in PRONOUNS:
            errors.append(f'{where}: "pronoun" must be "he", "she" or "they".')
        for k in keys:
            if not isinstance(s.get(k), str) or not s[k].strip():
                errors.append(f'{where}: missing "{k}".')
        profile = tuple(str(s.get(k, "")).strip().lower() for k in keys)
        if use_pronouns:
            profile = (s.get("pronoun"),) + profile
        if profile in profiles:
            errors.append(f"{where}: same {'pronoun and ' if use_pronouns else ''}traits as "
                          f"{profiles[profile]}, so the crime computer could never tell them "
                          "apart. Change a trait.")
        profiles[profile] = name
    if use_pronouns:
        for p in PRONOUNS:
            alone = [s["name"] for s in suspects if isinstance(s, dict) and s.get("pronoun") == p]
            if len(alone) == 1:
                warnings.append(f'suspects.json: {alone[0]} is the only "{p}", so witnesses saying '
                                f'"{p}" will give them away. Add another "{p}", or set '
                                '"witness_pronouns" to false so witnesses say "they" for everyone.')
    for k in keys:
        values = {str(s.get(k, "")).strip().lower() for s in suspects if isinstance(s, dict)}
        if len(suspects) > 2 and len(values) == 1:
            warnings.append(f'suspects.json: everyone has the same "{k}", so that clue never helps.')
    return errors, warnings


def load_data(folder):
    cdoc = read_json(os.path.join(folder, "countries.json"))
    sdoc = read_json(os.path.join(folder, "suspects.json"))
    errors, warnings = validate(cdoc, sdoc)
    if errors:
        return None, errors, warnings
    for s in sdoc["suspects"]:
        for t in sdoc["traits"]:
            s[t["key"]] = s[t["key"]].strip()
    data = {"countries": cdoc["countries"], "traits": sdoc["traits"],
            "suspects": sdoc["suspects"], "witness_pronouns": sdoc.get("witness_pronouns", True)}
    return data, errors, warnings

# --------------------------------------------------------------- the game


class Game:
    def __init__(self, countries, traits, suspects, difficulty, rng,
                 out=print, detective="Detective", witness_pronouns=True):
        self.countries, self.traits, self.suspects = countries, traits, suspects
        self.diff, self.legs = difficulty, difficulty["legs"]
        self.rng, self.out, self.detective = rng, out, detective
        self.use_pronouns = witness_pronouns

        self.thief = rng.choice(suspects)
        self.voice = self.thief["pronoun"] if witness_pronouns else "they"
        self.route = rng.sample(countries, self.legs + 1)
        self.on_route = {c["country"]: i for i, c in enumerate(self.route)}
        self.start = self.route[0]
        self.treasure = rng.choice(self.start["treasures"])
        self.needed = self._needed_traits()
        self.trait_plan = self._plan_traits()
        self.hideout = rng.randrange(3)
        self.deadline = self._deadline()

        self.t = 0
        self.here = self.start
        self.progress = 0                 # index of the furthest route city reached
        self.places, self.statements, self.visited, self.decoys = {}, {}, {}, {}
        self.notes = []                   # (city label, witness, statement)
        self.warrant = {t["key"]: None for t in traits}
        if witness_pronouns:
            self.warrant = {"pronoun": None, **self.warrant}
        self.warrant_for = None
        self.over = self.won = False
        self.flights = self.wrong_turns = 0

    # ---- case planning

    def _needed_traits(self):
        """The traits that single out the thief (on top of the pronoun), found greedily."""
        keys = [t["key"] for t in self.traits]
        self.rng.shuffle(keys)
        pool = [s for s in self.suspects
                if not self.use_pronouns or s["pronoun"] == self.thief["pronoun"]]
        needed = []
        while len(pool) > 1 and keys:
            best = min(keys, key=lambda k: sum(same(s[k], self.thief[k]) for s in pool))
            keys.remove(best)
            needed.append(best)
            pool = [s for s in pool if same(s[best], self.thief[best])]
        return needed

    def _plan_traits(self):
        """Decide which trait each trail city's witnesses will mention."""
        keys = [t["key"] for t in self.traits]
        slots = [1] * self.legs
        target = min(2 * self.legs, max(len(keys), len(self.needed)) + 1)
        while sum(slots) < target:
            slots[self.rng.choice([i for i, n in enumerate(slots) if n < 2])] += 1
        seq = keys[:]
        self.rng.shuffle(seq)
        extra = self.needed[:] or keys[:]
        while len(seq) < sum(slots):
            self.rng.shuffle(extra)
            seq += extra
        plan, pos = [], 0
        for n in slots:
            chunk = seq[pos:pos + n]
            pos += n
            if len(chunk) == 2 and chunk[0] == chunk[1]:
                others = [k for k in keys if k != chunk[0]]
                if others:
                    chunk[1] = self.rng.choice(others)
                else:
                    chunk = chunk[:1]
            plan.append(chunk)
        return plan

    def _deadline(self):
        """Time for a careful player (every witness, a few wrong turns) plus a buffer."""
        legs = [flight_hours(self.route[i], self.route[i + 1]) for i in range(self.legs)]
        avg = round(sum(legs) / len(legs))
        actions = []
        for i in range(self.legs):
            actions += INTERVIEW_HOURS
            if i < self.diff["spare_detours"]:
                actions += [avg, INTERVIEW_HOURS[0], avg]
            actions.append(legs[i])
        actions += [WARRANT_HOURS, *INTERVIEW_HOURS]
        t = 0
        for n, h in enumerate(actions):
            t += h
            if n < len(actions) - 1:
                t = after_sleep(t)
        t += self.diff["buffer"]
        if after_sleep(t) != t:           # never set a deadline in the middle of the night
            t = after_sleep(t) + 2
        return t

    # ---- what witnesses say

    def places_in(self, c):
        if c["country"] not in self.places:
            self.places[c["country"]] = self.rng.sample(PLACES, 3)
        return self.places[c["country"]]

    def _trait_sentence(self, key):
        trait = next(t for t in self.traits if t["key"] == key)
        return fill(self.rng.choice(trait["clues"]), self.voice, self.thief[key])

    def _pick_clues(self, country, places):
        pool = [(cat, text) for cat, texts in country["clues"].items() for text in texts]
        self.rng.shuffle(pool)
        picked, used = [], set()
        for _, _, prefers in places:
            for tier in ([p for p in pool if p[0] in prefers and p[0] not in used],
                         [p for p in pool if p[0] not in used],
                         pool):
                if tier:
                    choice = tier[0]
                    break
            pool.remove(choice)
            used.add(choice[0])
            picked.append(choice[1])
        return picked

    def _statements(self, c):
        name = c["country"]
        if name in self.statements:
            return self.statements[name]
        places, idx, p = self.places_in(c), self.on_route.get(name), self.voice
        if idx is None:                                   # wrong city
            lines = self.rng.sample(NOBODY, 3)
        elif idx == self.legs:                            # the hideout city
            traits = self.needed[:] or [t["key"] for t in self.traits]
            self.rng.shuffle(traits)
            warns = self.rng.sample(WARNINGS, 3)
            lines = []
            for i in range(3):
                if i == self.hideout:
                    lines.append(None)
                    continue
                said = [fill(warns[i], p)]
                if traits:
                    said.append(self._trait_sentence(traits.pop()))
                lines.append(" ".join(said))
        else:                                             # on the trail
            clues = self._pick_clues(self.route[idx + 1], places)
            plan = self.trait_plan[idx]
            who = self.rng.sample(range(3), len(plan))
            lines = []
            for i in range(3):
                said = [fill(clues[i], p)]
                if i in who:
                    extra = self._trait_sentence(plan[who.index(i)])
                    said.insert(self.rng.randrange(2), extra)
                lines.append(" ".join(said))
        self.statements[name] = lines
        return lines

    def destinations(self):
        name = self.here["country"]
        idx = self.on_route.get(name)
        if name not in self.decoys:
            pool = [x for x in self.countries
                    if x["country"] not in self.on_route and x["country"] != name]
            self.decoys[name] = self.rng.sample(pool, 4 if idx == self.legs else 3)
        dests = list(self.decoys[name])
        if idx is None:
            dests.append(self.route[self.progress])       # the way back to the trail
        elif idx < self.legs:
            dests.append(self.route[idx + 1])             # the right answer
        return sorted(dests, key=lambda d: d["city"])

    def next_interview_hours(self):
        done = self.visited.get(self.here["country"], [])
        return INTERVIEW_HOURS[min(len(done), len(INTERVIEW_HOURS) - 1)]

    # ---- actions

    def _spend(self, hours):
        self.t += hours
        if self.t > self.deadline:
            self._time_up()
            return False
        return True

    def _night(self):
        if self.over:
            return
        woke = after_sleep(self.t)
        if woke != self.t:
            self.t = woke
            if self.t > self.deadline:
                self._time_up(asleep=True)
            else:
                self.out(dim("  It's late. You check into a hotel and sleep until 07:00."))

    def arrive_text(self):
        self.out("")
        self.out(bold(f"  {place_label(self.here).upper()}") + dim(f"   {clock_text(self.t)}"))
        if self.here.get("about"):
            self.out(wrap(self.here["about"], "  "))

    def travel(self, dest):
        hours = flight_hours(self.here, dest)
        self.out(dim(f"  You fly from {self.here['city']} to {dest['city']} ({hours} hours)."))
        if not self._spend(hours):
            return
        self.here = dest
        self.flights += 1
        idx = self.on_route.get(dest["country"])
        if idx == self.progress + 1:
            self.progress = idx
        elif idx is None:
            self.wrong_turns += 1
        self.arrive_text()
        self._night()

    def investigate(self, i):
        c = self.here
        place, witness, _ = self.places_in(c)[i]
        done = self.visited.setdefault(c["country"], [])
        lines = self._statements(c)
        if i in done:
            self.out(dim(f"  You've already been to the {place}. From your notes:"))
            self.out(wrap(f'{witness}: "{lines[i]}"', "  ", "    "))
            return
        if not self._spend(self.next_interview_hours()):
            return
        done.append(i)
        if self.on_route.get(c["country"]) == self.legs and i == self.hideout:
            self._showdown(place)
            return
        self.out(dim(f"  {clock_text(self.t)} at the {place}"))
        self.out(wrap(f'{cyan(witness)}: "{lines[i]}"', "  ", "    "))
        self.notes.append((place_label(c), witness, lines[i]))
        self._night()

    def fits(self, s):
        return all(v is None or same(s[k], v) for k, v in self.warrant.items())

    def compute_warrant(self):
        if all(v is None for v in self.warrant.values()):
            self.out("  Enter at least one detail first.")
            return
        if not self._spend(WARRANT_HOURS):
            return
        had = self.warrant_for
        matches = [s for s in self.suspects if self.fits(s)]
        if len(matches) == 1:
            self.warrant_for = matches[0]
            self.out(green(f"  Warrant issued for {matches[0]['name']}!"))
        else:
            self.warrant_for = None
            if not matches:
                self.out(yellow("  Nobody on file matches that description. Check your details."))
            else:
                self.out(yellow("  Possible suspects: " + ", ".join(s["name"] for s in matches)))
                self.out("  Add more details to narrow it down to one.")
            if had:
                self.out(dim(f"  (Your warrant for {had['name']} has been withdrawn.)"))
        self._night()

    # ---- endings

    def _reveal(self):
        return f"The thief was {self.thief['name']}."

    def _time_up(self, asleep=False):
        self.over = True
        self.t = max(self.t, self.deadline)
        self.out("")
        when = "While you slept, the" if asleep else "The"
        self.out(red(f"  TIME'S UP! {when} deadline passed."))
        self.out(wrap(f"{self.thief['name']} got away with {self.treasure}.", "  "))

    def _showdown(self, place):
        self.over = True
        thief = self.thief
        self.out("")
        self.out(red(wrap(f"!!! At the {place}, "
                          + self.rng.choice(SPOTTED).format(name=thief["name"]), "  ")))
        if self.warrant_for is None:
            self.out(wrap(f"...but you have no warrant! {fill('{They}', thief['pronoun'])} "
                          "gives you a cheeky wave and disappears into the crowd.", "  "))
            self.out(red("  CASE LOST."))
        elif self.warrant_for is not thief:
            self.out(wrap(f"...but your warrant is for {self.warrant_for['name']}! "
                          f"{thief['name']} laughs and strolls off.", "  "))
            self.out(red("  CASE LOST."))
        else:
            self.won = True
            self.out("  You show your warrant and snap on the handcuffs.")
            if thief.get("catchphrase"):
                self.out(wrap(f'{thief["name"]}: "{thief["catchphrase"]}"', "  ", "    "))
            self.out("")
            self.out(green("  CASE CLOSED!"))
            self.out(wrap(f"You recover {self.treasure} and send it home to "
                          f"{self.start['city']}. Nice work, Detective {self.detective}.", "  "))
        self.out(dim(f"  Time taken: {duration_text(self.t)}   Flights: {self.flights}"
                     f"   Wrong turns: {self.wrong_turns}"))

    # ---- interactive play

    def play(self):
        self.briefing()
        while not self.over:
            self.status()
            choice = ask().lower()
            if choice in ("1", "i"):
                self.ui_investigate()
            elif choice in ("2", "t"):
                self.ui_travel()
            elif choice in ("3", "w", "c"):
                self.ui_warrant()
            elif choice in ("4", "n"):
                self.ui_notes()
            elif choice in ("5", "d"):
                self.ui_dossiers()
            elif choice in ("6", "h", "?"):
                show_help()
            elif choice in ("q", "quit", "exit"):
                if ask("  Give up this case? [y/n] ").lower().startswith("y"):
                    self.over = True
                    self.out(wrap(f"You hand in your badge for this one. {self._reveal()}", "  "))
            elif choice:
                self.out("  Pick an option from the menu.")

    def briefing(self):
        who = {"he": "a man", "she": "a woman", "they": "someone"}[self.voice]
        self.out("")
        self.out(bold(f"  === {AGENCY.upper()} ==="))
        self.out("")
        self.out(yellow(wrap(f"FLASH! Stolen from {place_label(self.start)}: "
                             f"{self.treasure}!", "  ")))
        self.out("")
        self.out(wrap(f"Witnesses saw {who} fleeing the scene, and our files say "
                      "it's someone in the family.", "  "))
        self.out("")
        self.out(wrap(f"Your assignment, Detective {self.detective}: follow the trail, "
                      "get a warrant from the crime computer, and make the arrest before "
                      f"{clock_text(self.deadline)}.", "  "))
        ask(dim("\n  Press Enter to begin. "))
        self.arrive_text()

    def status(self):
        left = self.deadline - self.t
        warrant = self.warrant_for["name"] if self.warrant_for else "none"
        self.out("")
        self.out(dim("  " + "-" * (WIDTH - 2)))
        self.out(f"  {bold(place_label(self.here))}  {dim('|')}  {clock_text(self.t)}"
                 f"  {dim('|')}  {left} hours left")
        self.out(dim(f"  Deadline {clock_text(self.deadline)}   Warrant: {warrant}"))
        self.out("  [1] Investigate  [2] Travel  [3] Crime computer  [4] Notes")
        self.out("  [5] Dossiers     [6] Help    [Q] Quit")

    def ui_investigate(self):
        places = self.places_in(self.here)
        done = self.visited.get(self.here["country"], [])
        self.out("")
        self.out(f"  Where do you want to investigate? "
                 + dim(f"(a new interview takes {self.next_interview_hours()} h)"))
        for n, (place, _, _) in enumerate(places, 1):
            mark = dim("  (visited)") if n - 1 in done else ""
            self.out(f"   [{n}] {place}{mark}")
        self.out("   [0] Back")
        pick = choose(len(places))
        if pick is not None:
            self.out("")
            self.investigate(pick)

    def ui_travel(self):
        dests = self.destinations()
        self.out("")
        self.out(f"  Flights from {self.here['city']}:")
        for n, d in enumerate(dests, 1):
            label = place_label(d)
            self.out(f"   [{n}] {label:<34}" + dim(f"{flight_hours(self.here, d):>3} h"))
        self.out("   [0] Back")
        pick = choose(len(dests))
        if pick is not None:
            self.out("")
            self.travel(dests[pick])

    def warrant_fields(self):
        fields = [("pronoun", "Pronoun", sorted(PRONOUNS))] if self.use_pronouns else []
        for t in self.traits:
            values = []
            for s in self.suspects:
                if not any(same(s[t["key"]], v) for v in values):
                    values.append(s[t["key"]])
            fields.append((t["key"], t["label"], sorted(values, key=str.lower)))
        return fields

    def ui_warrant(self):
        fields = self.warrant_fields()
        while not self.over:
            self.out("")
            self.out(bold("  CRIME COMPUTER") + dim("  enter what you know, then compute"))
            for n, (key, label, _) in enumerate(fields, 1):
                value = self.warrant[key] or dim("?")
                self.out(f"   [{n}] {label + ' ':.<22} {value}")
            self.out(f"   [C] Compute warrant " + dim(f"({WARRANT_HOURS} h)"))
            self.out("   [R] Clear all   [0] Back")
            current = self.warrant_for["name"] if self.warrant_for else "none"
            self.out(dim(f"  Current warrant: {current}"))
            choice = ask().lower()
            if choice in ("0", "", "b"):
                return
            if choice == "c":
                self.out("")
                self.compute_warrant()
                if self.warrant_for:
                    return
            elif choice == "r":
                self.warrant = {k: None for k in self.warrant}
            elif choice.isdigit() and 1 <= int(choice) <= len(fields):
                key, label, values = fields[int(choice) - 1]
                self.out(f"  {label}:")
                for n, v in enumerate(values, 1):
                    self.out(f"   [{n}] {v}")
                self.out("   [0] Don't know")
                pick = ask()
                if pick == "0":
                    self.warrant[key] = None
                elif pick.isdigit() and 1 <= int(pick) <= len(values):
                    self.warrant[key] = values[int(pick) - 1]

    def ui_notes(self):
        self.out("")
        self.out(bold("  CASE NOTES") + dim(f"  {sentence_case(self.treasure)}, "
                                           f"stolen from {self.start['city']}"))
        if not self.notes:
            self.out("  Nothing yet. Go and investigate!")
            return
        last = None
        for city, witness, text in self.notes:
            if city != last:
                self.out("")
                self.out(f"  {bold(city)}")
                last = city
            self.out(wrap(f'{witness}: "{text}"', "    ", "      "))

    def ui_dossiers(self):
        self.out("")
        self.out(bold("  DOSSIERS") + dim(f"  {len(self.suspects)} people on file"))
        for s in self.suspects:
            self.out("")
            self.out(f"  {bold(s['name'])} " + dim(f"({s['pronoun']})"))
            facts = " · ".join(f"{t['label']}: {s[t['key']]}" for t in self.traits)
            self.out(wrap(facts, "    "))
            if s.get("bio"):
                self.out(dim(wrap(s["bio"], "    ")))
        ask(dim("\n  Press Enter to go back. "))

# ------------------------------------------------------------ menus etc.


def ask(prompt="  > "):
    try:
        return input(prompt).strip()
    except EOFError:
        print()
        raise SystemExit(0)


def choose(count):
    """Read a 1-based menu choice; return a 0-based index or None for Back."""
    while True:
        pick = ask()
        if pick in ("", "0", "b"):
            return None
        if pick.isdigit() and 1 <= int(pick) <= count:
            return int(pick) - 1
        print(f"  Type a number from 1 to {count}, or 0 to go back.")


def show_help():
    print()
    for para in (
        "HOW TO PLAY",
        "A family member has stolen a treasure and is hopping around the world. "
        "Follow the trail to their hideout and arrest them before the deadline.",
        "INVESTIGATE: each city has three places with a witness. On the right "
        "trail, witnesses hint at the next country (flags, money, landmarks, food, "
        "language...) and sometimes describe the thief. In a wrong city nobody has "
        "seen a thing, so fly back. Each new interview in a city costs more time "
        f"({', '.join(map(str, INTERVIEW_HOURS))} hours).",
        "TRAVEL: four flights leave every city. Longer flights take longer.",
        "CRIME COMPUTER: enter what you've learned about the thief and compute "
        f"({WARRANT_HOURS} hour). Once only one person fits, you get a warrant. "
        "Check the dossiers to match clues to people.",
        "THE HIDEOUT: when witnesses say the thief is still in town, you're close. "
        "Keep investigating to catch them, but you need a warrant for the right "
        "person or they walk free.",
        "TIME: the clock runs from Monday 09:00. You sleep from 23:00 to 07:00.",
    ):
        print(wrap(para, "  "))
        print()
    ask(dim("  Press Enter to go back. "))


def title():
    line = "=" * (WIDTH - 2)
    print()
    print(bold("  " + line))
    print(bold(f"  {GAME_TITLE:^{WIDTH - 2}}"))
    print(dim(f"  {'a family-and-friends detective chase':^{WIDTH - 2}}"))
    print(bold("  " + line))
    print()


def pick_difficulty(preset):
    if preset:
        return DIFFICULTIES[int(preset) - 1]
    print("  Choose your difficulty:")
    for n, d in enumerate(DIFFICULTIES, 1):
        print(f"   [{n}] {d['name']:<13}" + dim(f"{d['legs']} flights to the hideout"))
    while True:
        pick = ask()
        if pick in ("1", "2", "3"):
            return DIFFICULTIES[int(pick) - 1]
        if pick.lower() in ("q", "quit"):
            raise SystemExit(0)
        print("  Type 1, 2 or 3.")


def setup_terminal():
    try:
        sys.stdout.reconfigure(errors="replace")
    except (AttributeError, ValueError):
        pass
    if os.name == "nt":
        os.system("")                     # switches on colour codes in Windows terminals
    Colour.on = sys.stdout.isatty() and "NO_COLOR" not in os.environ


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    ap = argparse.ArgumentParser(description=f"{GAME_TITLE} A detective chase in your terminal.")
    ap.add_argument("--data", default=here,
                    help="folder holding countries.json and suspects.json (default: next to this script)")
    ap.add_argument("--seed", type=int, help="replay the same case by using the same number")
    ap.add_argument("--difficulty", choices=("1", "2", "3"), help="1 Rookie, 2 Detective, 3 Super Sleuth")
    ap.add_argument("--check", action="store_true", help="check the data files and exit")
    args = ap.parse_args()
    setup_terminal()

    try:
        data, errors, warnings = load_data(args.data)
    except DataError as e:
        print(f"Problem reading the data files:\n  {e}")
        return 1
    if errors:
        print("Please fix these problems in the data files:")
        for e in errors:
            print(wrap(e, "  - ", "    "))
        return 1
    if args.check:
        clues = sum(len(v) for c in data["countries"] for v in c["clues"].values())
        print(f"Data looks good: {len(data['countries'])} countries, {clues} clues, "
              f"{len(data['suspects'])} suspects, {len(data['traits'])} traits.")
        for w in warnings:
            print(wrap(w, "  ! ", "    "))
        return 0

    rng = random.Random(args.seed)
    try:
        title()
        name = ask("  What's your name, detective? ") or "Detective"
        while True:
            print()
            game = Game(difficulty=pick_difficulty(args.difficulty), rng=rng,
                        detective=name, **data)
            game.play()
            if not ask("\n  Take another case? [y/n] ").lower().startswith("y"):
                break
        print("\n  Thanks for playing!\n")
    except KeyboardInterrupt:
        print("\n\n  Case abandoned. See you next time!\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
