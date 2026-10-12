//! The game rules, with no drawing code. Everything the player should read
//! goes into `Game::log`, so the same logic can run headless (tests,
//! simulations) or behind any front end.

use std::collections::HashMap;
use std::rc::Rc;

use crate::data::{Country, Data, PRONOUNS};
use crate::rng::Rng;

pub const GAME_TITLE: &str = "WHERE IN THE WORLD?";
pub const AGENCY: &str = "Family Bureau of Investigation";

pub const START_HOUR: i64 = 9; // every case starts Monday 09:00
const BEDTIME: i64 = 23; // detectives sleep from 23:00 to 07:00
const WAKE_UP: i64 = 7;
pub const INTERVIEW_HOURS: [i64; 3] = [1, 2, 3]; // 1st, 2nd and 3rd interview in the same city
pub const WARRANT_HOURS: i64 = 1; // each run of the crime computer
const FLIGHT_KMH: f64 = 800.0; // speed used to turn distance into flight time
const AIRPORT_HOURS: i64 = 1; // check-in, taxis and so on, added to each flight

#[derive(Clone, Copy)]
pub struct Difficulty {
    pub name: &'static str,
    /// Flights from the crime scene to the hideout.
    pub legs: usize,
    /// Roughly how many wrong turns you can afford.
    pub spare_detours: usize,
    /// Extra hours on top of that.
    pub buffer: i64,
}

pub const DIFFICULTIES: [Difficulty; 3] = [
    Difficulty { name: "Rookie", legs: 4, spare_detours: 2, buffer: 12 },
    Difficulty { name: "Detective", legs: 5, spare_detours: 1, buffer: 6 },
    Difficulty { name: "Super Sleuth", legs: 6, spare_detours: 0, buffer: 2 },
];

pub struct Place {
    pub name: &'static str,
    pub witness: &'static str,
    /// Clue types this witness tends to mention.
    prefers: &'static [&'static str],
}

const fn place(name: &'static str, witness: &'static str, prefers: &'static [&'static str]) -> Place {
    Place { name, witness, prefers }
}

pub static PLACES: [Place; 9] = [
    place("Bank", "Bank teller", &["currency"]),
    place("Airport", "Check-in agent", &["flag", "geography"]),
    place("Library", "Librarian", &["language", "landmark"]),
    place("Museum", "Museum guide", &["landmark", "culture"]),
    place("Market", "Market trader", &["culture", "currency"]),
    place("Hotel", "Hotel concierge", &[]),
    place("Bus station", "Ticket seller", &["geography", "landmark"]),
    place("Café", "Barista", &["culture", "language"]),
    place("Taxi rank", "Taxi driver", &["landmark", "geography"]),
];

// What witnesses say when the thief never came through their city.
const NOBODY: [&str; 6] = [
    "Sorry, I haven't seen anyone like that around here.",
    "Nope. Nobody like that came through today.",
    "I'd remember a face like that. Haven't seen one.",
    "Never seen anyone matching that description, sorry.",
    "Quiet day. Nobody unusual at all.",
    "Can't help you, detective. Nobody like that.",
];

// What witnesses say in the hideout city.
const WARNINGS: [&str; 4] = [
    "Careful, detective! Someone like that rushed past here not an hour ago. {They} must still be in town!",
    "{They} asked me about good places to hide around here. Watch your step!",
    "I saw {them} a moment ago, looking over {their} shoulder. {They} can't have gone far!",
    "{They} said {they} planned to lie low in this city for a while. Hurry!",
];

const SPOTTED: [&str; 3] = [
    "you spot {name} trying to slip out of the back door!",
    "you spot {name} hiding behind a newspaper with two eye-holes cut in it!",
    "you spot {name} tiptoeing away with a suspiciously heavy bag!",
];

const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

// ------------------------------------------------------------ text helpers

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tone {
    Plain,
    Dim,
    Heading,
    Good,
    Warn,
    Bad,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub tone: Tone,
    /// Set for witness statements, so a front end can style the name.
    pub speaker: Option<String>,
    pub text: String,
}

pub fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Swap {They}/{them}/{their}/{value} placeholders for real words.
pub fn fill(template: &str, pronoun: &str, value: Option<&str>) -> String {
    let (they, them, their) = match pronoun {
        "he" => ("he", "him", "his"),
        "she" => ("she", "her", "her"),
        _ => ("they", "them", "their"),
    };
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let len = after.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(after.len());
        let closed = len > 0 && after[len..].starts_with('}');
        let sub = match &after[..len] {
            _ if !closed => None,
            "they" => Some(they.to_string()),
            "them" => Some(them.to_string()),
            "their" => Some(their.to_string()),
            "They" => Some(capitalise(they)),
            "Them" => Some(capitalise(them)),
            "Their" => Some(capitalise(their)),
            "value" => value.map(str::to_string),
            _ => None,
        };
        match sub {
            Some(s) => {
                out.push_str(&s);
                rest = &after[len + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    capitalise(&out)
}

/// In the original's style: "Monday, 9 a.m.".
pub fn clock_text(t: i64) -> String {
    let day = (START_HOUR + t).div_euclid(24);
    let name = DAYS[day.rem_euclid(7) as usize];
    let next = if day >= 7 { "next " } else { "" };
    let time = match hour_of_day(t) {
        0 => "midnight".to_string(),
        12 => "noon".to_string(),
        h if h < 12 => format!("{h} a.m."),
        h => format!("{} p.m.", h - 12),
    };
    format!("{next}{name}, {time}")
}

fn with_full_stop(mut text: String) -> String {
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}

pub fn hour_of_day(t: i64) -> i64 {
    (START_HOUR + t).rem_euclid(24)
}

pub fn duration_text(hours: i64) -> String {
    let (days, hrs) = (hours / 24, hours % 24);
    let plural = |n: i64| if n == 1 { "" } else { "s" };
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days} day{}", plural(days)));
    }
    if hrs > 0 || days == 0 {
        parts.push(format!("{hrs} hour{}", plural(hrs)));
    }
    parts.join(", ")
}

/// If it's night at time t, return the time you wake up; otherwise t.
fn after_sleep(t: i64) -> i64 {
    let hour = (START_HOUR + t).rem_euclid(24);
    if hour >= BEDTIME {
        t + (24 - hour) + WAKE_UP
    } else if hour < WAKE_UP {
        t + (WAKE_UP - hour)
    } else {
        t
    }
}

fn km_between(a: &Country, b: &Country) -> f64 {
    let (lat1, lon1, lat2, lon2) = (a.lat.to_radians(), a.lon.to_radians(), b.lat.to_radians(), b.lon.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    2.0 * 6371.0 * h.sqrt().asin()
}

pub fn flight_hours(a: &Country, b: &Country) -> i64 {
    // round_ties_even matches Python's round(), which the timings were calibrated with.
    let flying = (km_between(a, b) / FLIGHT_KMH).round_ties_even() as i64;
    (flying + AIRPORT_HOURS).max(2)
}

pub fn same(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

// --------------------------------------------------------------- the game

pub struct Game {
    pub data: Rc<Data>,
    pub diff: Difficulty,
    rng: Rng,
    /// How the briefing addresses the player, e.g. "Detective Sam".
    detective: String,
    voice: String,
    pub thief: usize,
    /// Country indices from the crime scene to the hideout.
    route: Vec<usize>,
    on_route: HashMap<usize, usize>,
    pub start: usize,
    pub treasure: String,
    needed: Vec<String>,
    trait_plan: Vec<Vec<String>>,
    hideout: usize,
    pub deadline: i64,

    pub t: i64,
    pub here: usize,
    progress: usize, // index of the furthest route city reached
    places: HashMap<usize, [usize; 3]>,
    statements: HashMap<usize, Vec<Option<String>>>,
    visited: HashMap<usize, Vec<usize>>,
    decoys: HashMap<usize, Vec<usize>>,
    /// Crime computer entries, in display order: (key, value or unknown).
    pub warrant: Vec<(String, Option<String>)>,
    pub warrant_for: Option<usize>,
    pub over: bool,
    pub won: bool,
    /// The case ended with the thief cornered (rightly arrested or not), not with time running out.
    pub showdown: bool,
    pub flights: u32,
    pub wrong_turns: u32,
    /// Every city the player has been to, in order, for drawing the journey.
    pub path: Vec<usize>,
    pub log: Vec<Line>,
}

impl Game {
    pub fn new(data: Rc<Data>, diff: Difficulty, mut rng: Rng, detective: &str) -> Game {
        let thief = rng.below(data.suspects.len());
        let voice = if data.witness_pronouns { data.suspects[thief].pronoun.clone() } else { "they".to_string() };
        let all: Vec<usize> = (0..data.countries.len()).collect();
        let route = rng.sample(&all, diff.legs + 1);
        let start = route[0];
        let treasure = rng.choice(&data.countries[start].treasures).clone();
        let mut warrant: Vec<(String, Option<String>)> = data.traits.iter().map(|t| (t.key.clone(), None)).collect();
        if data.witness_pronouns {
            warrant.insert(0, ("pronoun".to_string(), None));
        }
        let mut game = Game {
            on_route: route.iter().enumerate().map(|(i, &c)| (c, i)).collect(),
            data,
            diff,
            rng,
            detective: match detective.trim() {
                "" => "Detective".to_string(),
                name => format!("Detective {name}"),
            },
            voice,
            thief,
            route,
            start,
            treasure,
            needed: Vec::new(),
            trait_plan: Vec::new(),
            hideout: 0,
            deadline: 0,
            t: 0,
            here: start,
            progress: 0,
            places: HashMap::new(),
            statements: HashMap::new(),
            visited: HashMap::new(),
            decoys: HashMap::new(),
            warrant,
            warrant_for: None,
            over: false,
            won: false,
            showdown: false,
            flights: 0,
            wrong_turns: 0,
            path: vec![start],
            log: Vec::new(),
        };
        game.needed = game.needed_traits();
        game.trait_plan = game.plan_traits();
        game.hideout = game.rng.below(3);
        game.deadline = game.compute_deadline();
        game
    }

    pub fn country(&self, i: usize) -> &Country {
        &self.data.countries[i]
    }

    fn say(&mut self, tone: Tone, text: impl Into<String>) {
        self.log.push(Line { tone, speaker: None, text: text.into() });
    }

    fn speech(&mut self, speaker: &str, text: &str) {
        self.log.push(Line { tone: Tone::Plain, speaker: Some(speaker.to_string()), text: format!("\u{201c}{text}\u{201d}") });
    }

    // ---- case planning

    /// The traits that single out the thief (on top of the pronoun), found greedily.
    fn needed_traits(&mut self) -> Vec<String> {
        let data = Rc::clone(&self.data);
        let thief = &data.suspects[self.thief];
        let mut keys: Vec<String> = data.traits.iter().map(|t| t.key.clone()).collect();
        self.rng.shuffle(&mut keys);
        let mut pool: Vec<_> = data
            .suspects
            .iter()
            .filter(|s| !data.witness_pronouns || s.pronoun == thief.pronoun)
            .collect();
        let mut needed = Vec::new();
        while pool.len() > 1 && !keys.is_empty() {
            let matching = |k: &str| pool.iter().filter(|s| same(s.get(k), thief.get(k))).count();
            let best = (0..keys.len()).min_by_key(|&i| matching(&keys[i])).unwrap_or(0);
            let best = keys.remove(best);
            pool.retain(|s| same(s.get(&best), thief.get(&best)));
            needed.push(best);
        }
        needed
    }

    /// Decide which trait each trail city's witnesses will mention.
    fn plan_traits(&mut self) -> Vec<Vec<String>> {
        let legs = self.diff.legs;
        let keys: Vec<String> = self.data.traits.iter().map(|t| t.key.clone()).collect();
        let mut slots = vec![1usize; legs];
        let target = (2 * legs).min(keys.len().max(self.needed.len()) + 1);
        while slots.iter().sum::<usize>() < target {
            let open: Vec<usize> = (0..legs).filter(|&i| slots[i] < 2).collect();
            let i = *self.rng.choice(&open);
            slots[i] += 1;
        }
        let mut seq = keys.clone();
        self.rng.shuffle(&mut seq);
        let mut extra = if self.needed.is_empty() { keys.clone() } else { self.needed.clone() };
        let total: usize = slots.iter().sum();
        while seq.len() < total {
            self.rng.shuffle(&mut extra);
            seq.extend(extra.iter().cloned());
        }
        let mut plan = Vec::new();
        let mut pos = 0;
        for n in slots {
            let mut chunk = seq[pos..pos + n].to_vec();
            pos += n;
            if chunk.len() == 2 && chunk[0] == chunk[1] {
                let others: Vec<&String> = keys.iter().filter(|k| **k != chunk[0]).collect();
                if others.is_empty() {
                    chunk.truncate(1);
                } else {
                    chunk[1] = self.rng.choice(&others).to_string();
                }
            }
            plan.push(chunk);
        }
        plan
    }

    /// Time for a careful player (every witness, a few wrong turns) plus a buffer.
    fn compute_deadline(&self) -> i64 {
        let legs: Vec<i64> = (0..self.diff.legs)
            .map(|i| flight_hours(self.country(self.route[i]), self.country(self.route[i + 1])))
            .collect();
        let avg = (legs.iter().sum::<i64>() as f64 / legs.len() as f64).round_ties_even() as i64;
        let mut actions = Vec::new();
        for (i, &leg) in legs.iter().enumerate() {
            actions.extend(INTERVIEW_HOURS);
            if i < self.diff.spare_detours {
                actions.extend([avg, INTERVIEW_HOURS[0], avg]);
            }
            actions.push(leg);
        }
        actions.push(WARRANT_HOURS);
        actions.extend(INTERVIEW_HOURS);
        let mut t = 0;
        for (n, h) in actions.iter().enumerate() {
            t += h;
            if n < actions.len() - 1 {
                t = after_sleep(t);
            }
        }
        t += self.diff.buffer;
        if after_sleep(t) != t {
            // Never set a deadline in the middle of the night.
            t = after_sleep(t) + 2;
        }
        t
    }

    // ---- what witnesses say

    /// The three places (indices into PLACES) you can investigate in country `c`.
    pub fn places_in(&mut self, c: usize) -> [usize; 3] {
        if let Some(p) = self.places.get(&c) {
            return *p;
        }
        let all: Vec<usize> = (0..PLACES.len()).collect();
        let s = self.rng.sample(&all, 3);
        let picked = [s[0], s[1], s[2]];
        self.places.insert(c, picked);
        picked
    }

    fn trait_sentence(&mut self, key: &str) -> String {
        let data = Rc::clone(&self.data);
        let t = data.traits.iter().find(|t| t.key == key).expect("trait key from the data");
        let template = self.rng.choice(&t.clues);
        fill(template, &self.voice, Some(data.suspects[self.thief].get(key)))
    }

    fn pick_clues(&mut self, country: usize, places: [usize; 3]) -> Vec<String> {
        let data = Rc::clone(&self.data);
        let mut pool: Vec<(&str, &str)> = data.countries[country]
            .clues
            .iter()
            .flat_map(|(cat, texts)| texts.iter().map(move |t| (cat.as_str(), t.as_str())))
            .collect();
        self.rng.shuffle(&mut pool);
        let mut picked = Vec::new();
        let mut used: Vec<&str> = Vec::new();
        for p in places {
            let prefers = PLACES[p].prefers;
            let pos = pool
                .iter()
                .position(|(cat, _)| prefers.contains(cat) && !used.contains(cat))
                .or_else(|| pool.iter().position(|(cat, _)| !used.contains(cat)))
                .unwrap_or(0);
            let (cat, text) = pool.remove(pos);
            used.push(cat);
            picked.push(text.to_string());
        }
        picked
    }

    /// What the three witnesses in country `c` say. None marks the hideout.
    fn statements(&mut self, c: usize) -> Vec<Option<String>> {
        if let Some(lines) = self.statements.get(&c) {
            return lines.clone();
        }
        let places = self.places_in(c);
        let voice = self.voice.clone();
        let lines: Vec<Option<String>> = match self.on_route.get(&c).copied() {
            None => self.rng.sample(&NOBODY, 3).into_iter().map(|s| Some(s.to_string())).collect(),
            Some(idx) if idx == self.diff.legs => {
                let mut traits = if self.needed.is_empty() {
                    self.data.traits.iter().map(|t| t.key.clone()).collect()
                } else {
                    self.needed.clone()
                };
                self.rng.shuffle(&mut traits);
                let warns = self.rng.sample(&WARNINGS, 3);
                let mut lines = Vec::new();
                for (i, warn) in warns.iter().enumerate() {
                    if i == self.hideout {
                        lines.push(None);
                        continue;
                    }
                    let mut said = vec![fill(warn, &voice, None)];
                    if let Some(k) = traits.pop() {
                        said.push(self.trait_sentence(&k));
                    }
                    lines.push(Some(said.join(" ")));
                }
                lines
            }
            Some(idx) => {
                let clues = self.pick_clues(self.route[idx + 1], places);
                let plan = self.trait_plan[idx].clone();
                let who = self.rng.sample(&[0, 1, 2], plan.len());
                let mut lines = Vec::new();
                for (i, clue) in clues.iter().enumerate() {
                    let mut said = vec![fill(clue, &voice, None)];
                    if let Some(k) = who.iter().position(|&w| w == i) {
                        let extra = self.trait_sentence(&plan[k]);
                        let at = self.rng.below(2);
                        said.insert(at, extra);
                    }
                    lines.push(Some(said.join(" ")));
                }
                lines
            }
        };
        self.statements.insert(c, lines.clone());
        lines
    }

    /// The flights out of the current city, sorted by city name so the order gives nothing away.
    pub fn destinations(&mut self) -> Vec<usize> {
        let here = self.here;
        let idx = self.on_route.get(&here).copied();
        if !self.decoys.contains_key(&here) {
            let pool: Vec<usize> =
                (0..self.data.countries.len()).filter(|c| !self.on_route.contains_key(c) && *c != here).collect();
            let n = if idx == Some(self.diff.legs) { 4 } else { 3 };
            let picked = self.rng.sample(&pool, n);
            self.decoys.insert(here, picked);
        }
        let mut dests = self.decoys[&here].clone();
        match idx {
            None => dests.push(self.route[self.progress]), // the way back to the trail
            Some(i) if i < self.diff.legs => dests.push(self.route[i + 1]), // the right answer
            _ => {}
        }
        dests.sort_by(|a, b| self.country(*a).city.cmp(&self.country(*b).city));
        dests
    }

    /// Which of the current city's three places you've already investigated.
    pub fn visited_here(&self) -> &[usize] {
        self.visited.get(&self.here).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn next_interview_hours(&self) -> i64 {
        let done = self.visited_here().len();
        INTERVIEW_HOURS[done.min(INTERVIEW_HOURS.len() - 1)]
    }

    // ---- actions

    pub fn briefing(&mut self) -> Vec<Line> {
        let start = self.log.len();
        let who = match self.voice.as_str() {
            "he" => "a man",
            "she" => "a woman",
            _ => "someone",
        };
        self.say(Tone::Heading, format!("=== {} ===", AGENCY.to_uppercase()));
        self.say(Tone::Warn, format!("FLASH! Stolen from {}: {}!", self.country(self.start).label(), self.treasure));
        self.say(
            Tone::Plain,
            format!("Witnesses saw {who} fleeing the scene, and our files say it's someone in the family."),
        );
        self.say(
            Tone::Plain,
            format!(
                "Your assignment, {}: follow the trail, get a warrant from the crime computer, \
                 and make the arrest before {}",
                self.detective,
                // "Sunday, 9 a.m." already ends the sentence.
                with_full_stop(clock_text(self.deadline))
            ),
        );
        let lines = self.log[start..].to_vec();
        self.arrive_text();
        lines
    }

    fn arrive_text(&mut self) {
        let c = self.country(self.here);
        let (label, about) = (c.label().to_uppercase(), c.about.clone());
        self.say(Tone::Plain, "");
        self.say(Tone::Heading, label);
        self.say(Tone::Dim, clock_text(self.t));
        if let Some(about) = about {
            self.say(Tone::Plain, about);
        }
    }

    fn spend(&mut self, hours: i64) -> bool {
        self.t += hours;
        if self.t > self.deadline {
            self.time_up(false);
            return false;
        }
        true
    }

    fn night(&mut self) {
        if self.over {
            return;
        }
        let woke = after_sleep(self.t);
        if woke != self.t {
            self.t = woke;
            if self.t > self.deadline {
                self.time_up(true);
            } else {
                self.say(Tone::Dim, "It's late. You check into a hotel and sleep until 7 a.m.");
            }
        }
    }

    pub fn travel(&mut self, dest: usize) {
        if self.over {
            return;
        }
        let hours = flight_hours(self.country(self.here), self.country(dest));
        let line = format!("You fly from {} to {} ({hours} hours).", self.country(self.here).city, self.country(dest).city);
        self.say(Tone::Plain, "");
        self.say(Tone::Dim, line);
        if !self.spend(hours) {
            return;
        }
        self.here = dest;
        self.path.push(dest);
        self.flights += 1;
        match self.on_route.get(&dest).copied() {
            Some(idx) if idx == self.progress + 1 => self.progress = idx,
            None => self.wrong_turns += 1,
            _ => {}
        }
        self.arrive_text();
        self.night();
    }

    /// Interview the witness at place `i` (0, 1 or 2) in the current city.
    pub fn investigate(&mut self, i: usize) {
        if self.over {
            return;
        }
        let c = self.here;
        let place = &PLACES[self.places_in(c)[i]];
        let lines = self.statements(c);
        self.say(Tone::Plain, "");
        if self.visited_here().contains(&i) {
            self.say(Tone::Dim, format!("You've already been to the {}. From your notes:", place.name));
            self.speech(place.witness, lines[i].as_deref().unwrap_or_default());
            return;
        }
        if !self.spend(self.next_interview_hours()) {
            return;
        }
        self.visited.entry(c).or_default().push(i);
        let Some(statement) = &lines[i] else {
            self.showdown(place.name);
            return;
        };
        self.say(Tone::Dim, format!("{} at the {}", clock_text(self.t), place.name));
        self.speech(place.witness, statement);
        self.night();
    }

    pub fn fits(&self, suspect: usize) -> bool {
        let s = &self.data.suspects[suspect];
        self.warrant.iter().all(|(k, v)| v.as_ref().is_none_or(|v| same(s.get(k), v)))
    }

    pub fn compute_warrant(&mut self) {
        if self.over {
            return;
        }
        self.say(Tone::Plain, "");
        if self.warrant.iter().all(|(_, v)| v.is_none()) {
            self.say(Tone::Plain, "Enter at least one detail first.");
            return;
        }
        if !self.spend(WARRANT_HOURS) {
            return;
        }
        let had = self.warrant_for;
        let matches: Vec<usize> = (0..self.data.suspects.len()).filter(|&s| self.fits(s)).collect();
        if let [only] = matches[..] {
            self.warrant_for = Some(only);
            self.say(Tone::Good, format!("Warrant issued for {}!", self.data.suspects[only].name));
        } else {
            self.warrant_for = None;
            if matches.is_empty() {
                self.say(Tone::Warn, "Nobody on file matches that description. Check your details.");
            } else {
                let names: Vec<&str> = matches.iter().map(|&s| self.data.suspects[s].name.as_str()).collect();
                self.say(Tone::Warn, format!("Possible suspects: {}", names.join(", ")));
                self.say(Tone::Plain, "Add more details to narrow it down to one.");
            }
            if let Some(h) = had {
                let name = self.data.suspects[h].name.clone();
                self.say(Tone::Dim, format!("(Your warrant for {name} has been withdrawn.)"));
            }
        }
        self.night();
    }

    /// The crime computer's fields: (key, label, every value someone on file has).
    pub fn warrant_fields(&self) -> Vec<(String, String, Vec<String>)> {
        let mut fields = Vec::new();
        if self.data.witness_pronouns {
            let values = PRONOUNS.iter().map(|p| p.to_string()).collect();
            fields.push(("pronoun".to_string(), "Pronoun".to_string(), values));
        }
        for t in &self.data.traits {
            let mut values: Vec<String> = Vec::new();
            for s in &self.data.suspects {
                let v = s.get(&t.key);
                if !values.iter().any(|x| same(x, v)) {
                    values.push(v.to_string());
                }
            }
            values.sort_by_key(|v| v.to_lowercase());
            fields.push((t.key.clone(), t.label.clone(), values));
        }
        fields
    }

    pub fn give_up(&mut self) {
        if self.over {
            return;
        }
        self.over = true;
        let name = self.data.suspects[self.thief].name.clone();
        self.say(Tone::Plain, "");
        self.say(Tone::Plain, format!("You hand in your badge for this one. The thief was {name}."));
    }

    // ---- endings

    fn time_up(&mut self, asleep: bool) {
        self.over = true;
        self.t = self.t.max(self.deadline);
        let when = if asleep { "While you slept, the" } else { "The" };
        let name = self.data.suspects[self.thief].name.clone();
        self.say(Tone::Plain, "");
        self.say(Tone::Bad, format!("TIME'S UP! {when} deadline passed."));
        self.say(Tone::Plain, format!("{name} got away with {}.", self.treasure));
    }

    fn showdown(&mut self, place: &str) {
        self.over = true;
        self.showdown = true;
        let data = Rc::clone(&self.data);
        let thief = &data.suspects[self.thief];
        let spotted = self.rng.choice(&SPOTTED).replace("{name}", &thief.name);
        self.say(Tone::Bad, format!("!!! At the {place}, {spotted}"));
        match self.warrant_for {
            None => {
                let they = fill("{They}", &thief.pronoun, None);
                self.say(
                    Tone::Plain,
                    format!("...but you have no warrant! {they} gives you a cheeky wave and disappears into the crowd."),
                );
                self.say(Tone::Bad, "CASE LOST.");
            }
            Some(w) if w != self.thief => {
                self.say(
                    Tone::Plain,
                    format!("...but your warrant is for {}! {} laughs and strolls off.", data.suspects[w].name, thief.name),
                );
                self.say(Tone::Bad, "CASE LOST.");
            }
            Some(_) => {
                self.won = true;
                self.say(Tone::Plain, "You show your warrant and snap on the handcuffs.");
                if let Some(catchphrase) = &thief.catchphrase {
                    self.speech(&thief.name, catchphrase);
                }
                self.say(Tone::Good, "CASE CLOSED!");
                self.say(
                    Tone::Plain,
                    format!(
                        "You recover {} and send it home to {}. Nice work, {}.",
                        self.treasure,
                        self.country(self.start).city,
                        self.detective
                    ),
                );
            }
        }
        self.say(
            Tone::Dim,
            format!(
                "Time taken: {}   Flights: {}   Wrong turns: {}",
                duration_text(self.t),
                self.flights,
                self.wrong_turns
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn data() -> Rc<Data> {
        match crate::data::load(Some(Path::new(env!("CARGO_MANIFEST_DIR")))) {
            Ok((data, _)) => Rc::new(data),
            Err(_) => panic!("the data files have errors; run cargo run -- --check"),
        }
    }

    /// Follows the right route, interviews everyone, and fills in the warrant
    /// with the thief's full profile in the final city.
    fn play_perfectly(game: &mut Game) {
        for leg in 0..game.diff.legs {
            for i in 0..3 {
                game.investigate(i);
            }
            let next = game.route[leg + 1];
            assert!(game.destinations().contains(&next), "the right flight must be offered");
            game.travel(next);
        }
        let thief = &game.data.suspects[game.thief];
        let fields: Vec<_> = game.warrant.iter().map(|(k, _)| k.clone()).collect();
        game.warrant = fields.iter().map(|k| (k.clone(), Some(thief.get(k).to_string()))).collect();
        game.compute_warrant();
        for i in 0..3 {
            game.investigate(i);
        }
    }

    #[test]
    fn a_careful_player_on_the_right_trail_always_wins() {
        let data = data();
        for diff in DIFFICULTIES {
            for seed in 0..300 {
                let mut game = Game::new(Rc::clone(&data), diff, Rng::new(seed), "Test");
                play_perfectly(&mut game);
                assert!(game.won, "{} seed {seed} lost: {:?}", diff.name, game.log.last());
            }
        }
    }

    #[test]
    fn the_same_seed_replays_the_same_case() {
        let data = data();
        let run = || {
            let mut game = Game::new(Rc::clone(&data), DIFFICULTIES[1], Rng::new(42), "Test");
            game.briefing();
            play_perfectly(&mut game);
            game.log.iter().map(|l| l.text.clone()).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_warrant_for_the_wrong_person_loses() {
        let data = data();
        let mut game = Game::new(Rc::clone(&data), DIFFICULTIES[0], Rng::new(7), "Test");
        let innocent = (0..data.suspects.len()).find(|&s| s != game.thief).unwrap_or(0);
        for leg in 0..game.diff.legs {
            game.travel(game.route[leg + 1]);
        }
        game.warrant_for = Some(innocent);
        for i in 0..3 {
            game.investigate(i);
        }
        assert!(game.over && !game.won);
    }

    #[test]
    fn fills_pronouns_and_values() {
        assert_eq!(fill("{They} showed me {their} {value}.", "she", Some("bike")), "She showed me her bike.");
        assert_eq!(fill("{they} kept {them} {unknown}", "he", None), "He kept him {unknown}");
    }

    #[test]
    fn clock_and_durations() {
        assert_eq!(clock_text(0), "Monday, 9 a.m.");
        assert_eq!(clock_text(3), "Monday, noon");
        assert_eq!(clock_text(8), "Monday, 5 p.m.");
        assert_eq!(clock_text(15), "Tuesday, midnight");
        assert_eq!(clock_text(24 * 7), "next Monday, 9 a.m.");
        assert_eq!(duration_text(25), "1 day, 1 hour");
        assert_eq!(duration_text(0), "0 hours");
        assert_eq!(after_sleep(14), 22); // 23:00 Monday -> 07:00 Tuesday
    }
}
