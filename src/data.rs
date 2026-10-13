//! Reading and checking countries.json and suspects.json.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

use crate::game::RANKS;

pub const CLUE_TYPES: [&str; 6] = ["flag", "currency", "language", "landmark", "geography", "culture"];
/// A suspect's sex, as on the original's crime computer; witnesses say he or she.
pub const SEXES: [&str; 2] = ["female", "male"];
const PRONOUN_KEYS: [&str; 6] = ["They", "they", "Them", "them", "Their", "their"];
const RESERVED_KEYS: [&str; 5] = ["bio", "catchphrase", "name", "sex", "skin"];
/// Skin tones a suspect's dossier portrait can have.
pub const SKIN_TONES: [&str; 6] = ["light", "rosy", "olive", "tan", "brown", "dark"];

// Built into the binary so a release build can be handed to someone on its own.
const EMBEDDED_COUNTRIES: &str = include_str!("../countries.json");
const EMBEDDED_SUSPECTS: &str = include_str!("../suspects.json");

pub struct Country {
    pub country: String,
    pub city: String,
    pub lat: f64,
    pub lon: f64,
    pub about: Option<String>,
    pub treasures: Vec<String>,
    /// (clue type, sentences), in file order.
    pub clues: Vec<(String, Vec<String>)>,
}

impl Country {
    pub fn label(&self) -> String {
        format!("{}, {}", self.city, self.country)
    }
}

pub struct Trait {
    pub key: String,
    pub label: String,
    pub clues: Vec<String>,
    /// Whether the crime computer has it as a field. Traits it hasn't (like
    /// "Other") are only for witnesses to mention.
    pub computer: bool,
    /// How witnesses word some values, so the computer can say "Bald" while
    /// a witness says "no hair at all". Keys are lower case.
    pub phrases: HashMap<String, String>,
}

impl Trait {
    /// `value` as witnesses say it.
    pub fn phrase<'a>(&'a self, value: &'a str) -> &'a str {
        self.phrases.get(&value.to_lowercase()).map_or(value, String::as_str)
    }
}

pub struct Suspect {
    pub name: String,
    /// "female" or "male".
    pub sex: String,
    pub traits: HashMap<String, String>,
    pub bio: Option<String>,
    pub catchphrase: Option<String>,
    /// For the dossier portrait; picked from the name when not given.
    pub skin: Option<String>,
}

impl Suspect {
    /// A trait value, or the sex (the crime computer treats it as a field).
    pub fn get(&self, key: &str) -> &str {
        if key == "sex" { &self.sex } else { &self.traits[key] }
    }

    pub fn pronoun(&self) -> &'static str {
        if self.sex == "female" { "she" } else { "he" }
    }
}

pub struct Data {
    pub countries: Vec<Country>,
    pub traits: Vec<Trait>,
    pub suspects: Vec<Suspect>,
}

impl Data {
    pub fn clue_count(&self) -> usize {
        self.countries.iter().flat_map(|c| &c.clues).map(|(_, t)| t.len()).sum()
    }
}

pub enum LoadError {
    /// A file is missing or isn't valid JSON.
    Read(String),
    /// The JSON is readable but the content has problems.
    Invalid(Vec<String>),
}

/// Load from `folder`, or from the copies built into the binary when `folder` is None.
pub fn load(folder: Option<&Path>) -> Result<(Data, Vec<String>), LoadError> {
    let (ctext, stext) = match folder {
        Some(dir) => (read_file(&dir.join("countries.json"))?, read_file(&dir.join("suspects.json"))?),
        None => (EMBEDDED_COUNTRIES.to_string(), EMBEDDED_SUSPECTS.to_string()),
    };
    let cdoc = parse("countries.json", &ctext)?;
    let sdoc = parse("suspects.json", &stext)?;
    let (errors, warnings) = validate(&cdoc, &sdoc);
    if !errors.is_empty() {
        return Err(LoadError::Invalid(errors));
    }
    Ok((build(&cdoc, &sdoc), warnings))
}

fn read_file(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|e| LoadError::Read(format!("Can't read {}: {e}", path.display())))
}

fn parse(name: &str, text: &str) -> Result<Value, LoadError> {
    serde_json::from_str(text).map_err(|e| {
        let hint = if e.is_syntax() || e.is_eof() { "\n  (Usually a missing comma, quote or bracket just before that spot.)" } else { "" };
        LoadError::Read(format!("{name}: {e}.{hint}"))
    })
}

fn text(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.trim().is_empty())
}

fn text_list(v: Option<&Value>) -> Option<Vec<&str>> {
    v?.as_array()?.iter().map(|x| text(Some(x))).collect()
}

/// The names inside {curly} placeholders.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let len = after.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(after.len());
        if len > 0 && after[len..].starts_with('}') {
            out.push(&after[..len]);
            rest = &after[len + 1..];
        } else {
            rest = after;
        }
    }
    out
}

/// Whether `word` appears in `text` as a whole word, ignoring case.
fn names_word(text: &str, word: &str) -> bool {
    let (text, word) = (text.to_lowercase(), word.to_lowercase());
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let boundary = |a: Option<char>, b: Option<char>| is_word(a) != is_word(b);
    let (first, last) = (word.chars().next(), word.chars().next_back());
    text.match_indices(&word)
        .any(|(i, _)| boundary(text[..i].chars().next_back(), first) && boundary(last, text[i + word.len()..].chars().next()))
}

/// Return (errors, warnings) for the two data files.
pub fn validate(cdoc: &Value, sdoc: &Value) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // ---- countries.json
    let empty = Vec::new();
    let countries = match cdoc.get("countries").and_then(Value::as_array) {
        Some(list) => list,
        None => {
            errors.push(r#"countries.json needs a "countries" list."#.to_string());
            &empty
        }
    };
    let mut seen = HashSet::new();
    for (n, c) in countries.iter().enumerate() {
        let mut place = format!("countries.json, entry {}", n + 1);
        if !c.is_object() {
            errors.push(format!("{place}: should be an object {{ ... }}."));
            continue;
        }
        let name = text(c.get("country"));
        let city = text(c.get("city"));
        if let Some(name) = name {
            place = format!("countries.json, {name}");
        }
        for (key, v) in [("country", name), ("city", city)] {
            if v.is_none() {
                errors.push(format!(r#"{place}: missing "{key}"."#));
            }
        }
        for (key, lo, hi) in [("lat", -90.0, 90.0), ("lon", -180.0, 180.0)] {
            if !c.get(key).and_then(Value::as_f64).is_some_and(|v| (lo..=hi).contains(&v)) {
                errors.push(format!(r#"{place}: "{key}" should be a number from {lo} to {hi}."#));
            }
        }
        if !seen.insert(c.get("country").map(Value::to_string)) {
            errors.push(format!("{place}: listed twice."));
        }
        if !text_list(c.get("treasures")).is_some_and(|t| !t.is_empty()) {
            errors.push(format!(r#"{place}: needs a "treasures" list with at least one item."#));
        }
        let Some(clues) = c.get("clues").and_then(Value::as_object) else {
            errors.push(format!(r#"{place}: needs a "clues" section."#));
            continue;
        };
        let mut total = 0;
        for (cat, texts) in clues {
            let Some(texts) = text_list(Some(texts)) else {
                errors.push(format!(r#"{place}: clues "{cat}" should be a list of sentences."#));
                continue;
            };
            if !CLUE_TYPES.contains(&cat.as_str()) {
                warnings.push(format!(
                    r#"{place}: clue type "{cat}" is not one of {}. It still works, but no witness prefers it."#,
                    CLUE_TYPES.join(", ")
                ));
            }
            for t in texts {
                total += 1;
                let mut bad: Vec<_> = placeholders(t).into_iter().filter(|p| !PRONOUN_KEYS.contains(p)).collect();
                bad.sort();
                if let Some(b) = bad.first() {
                    errors.push(format!("{place}: unknown placeholder {{{b}}} in: {t}"));
                }
                if [name, city].into_iter().flatten().any(|g| names_word(t, g)) {
                    warnings.push(format!("{place}: this clue names the answer: {t}"));
                }
            }
        }
        if total < 3 {
            errors.push(format!("{place}: needs at least 3 clues (has {total})."));
        } else if total < 5 {
            warnings.push(format!("{place}: only {total} clues. Five or more keeps cases varied."));
        }
    }
    let need = RANKS.iter().map(|d| d.legs).max().unwrap_or(0) + 6;
    if countries.len() < need {
        errors.push(format!("countries.json has {} countries; the game needs at least {need}.", countries.len()));
    }

    // ---- suspects.json
    let traits = match sdoc.get("traits").and_then(Value::as_array) {
        Some(list) if !list.is_empty() => list,
        _ => {
            errors.push(r#"suspects.json needs a "traits" list."#.to_string());
            &empty
        }
    };
    let mut keys: Vec<&str> = Vec::new();
    for (n, t) in traits.iter().enumerate() {
        let Some(k) = text(t.get("key")) else {
            errors.push(format!(r#"suspects.json, trait {}: needs a "key"."#, n + 1));
            continue;
        };
        let place = format!(r#"suspects.json, trait "{k}""#);
        if keys.contains(&k) || RESERVED_KEYS.contains(&k) {
            errors.push(format!("{place}: that key is used twice or is reserved ({}).", RESERVED_KEYS.join(", ")));
        }
        keys.push(k);
        if text(t.get("label")).is_none() {
            errors.push(format!(r#"{place}: needs a "label" (shown in the crime computer)."#));
        }
        if t.get("computer").is_some_and(|c| !c.is_boolean()) {
            errors.push(format!(r#"{place}: "computer" must be true or false."#));
        }
        if let Some(phrases) = t.get("phrases")
            && !phrases.as_object().is_some_and(|m| m.values().all(|v| text(Some(v)).is_some()))
        {
            errors.push(format!(r#"{place}: "phrases" must map values to what witnesses say, like {{"bald": "no hair at all"}}."#));
        }
        let templates = t.get("clues").and_then(Value::as_array);
        let templates: Option<Vec<&str>> = templates.filter(|l| !l.is_empty()).and_then(|l| l.iter().map(Value::as_str).collect());
        let Some(templates) = templates else {
            errors.push(format!(r#"{place}: needs a "clues" list of witness sentences."#));
            continue;
        };
        for t in templates {
            let mut bad: Vec<_> = placeholders(t).into_iter().filter(|p| !PRONOUN_KEYS.contains(p) && *p != "value").collect();
            bad.sort();
            if let Some(b) = bad.first() {
                errors.push(format!("{place}: unknown placeholder {{{b}}} in: {t}"));
            }
            if !t.contains("{value}") {
                errors.push(format!("{place}: clue must contain {{value}}: {t}"));
            }
        }
    }

    // Only the crime computer's fields need to tell people apart.
    let searchable: Vec<&str> =
        traits.iter().filter(|t| t.get("computer").and_then(Value::as_bool) != Some(false)).filter_map(|t| text(t.get("key"))).collect();
    if searchable.is_empty() {
        errors.push(r#"suspects.json: at least one trait must be on the crime computer."#.to_string());
    }
    let suspects = match sdoc.get("suspects").and_then(Value::as_array) {
        Some(list) if list.len() >= 2 => list,
        _ => {
            errors.push(r#"suspects.json needs a "suspects" list with at least 2 people."#.to_string());
            &empty
        }
    };
    let value_of = |s: &Value, k: &str| match s.get(k) {
        Some(Value::String(v)) => v.trim().to_lowercase(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().to_lowercase(),
    };
    let mut names = HashSet::new();
    let mut profiles: HashMap<Vec<String>, &str> = HashMap::new();
    for (n, s) in suspects.iter().enumerate() {
        let place = format!("suspects.json, suspect {}", n + 1);
        if !s.is_object() {
            errors.push(format!("{place}: should be an object {{ ... }}."));
            continue;
        }
        let Some(name) = text(s.get("name")) else {
            errors.push(format!(r#"{place}: missing "name"."#));
            continue;
        };
        let place = format!("suspects.json, {name}");
        if !names.insert(name) {
            errors.push(format!("{place}: name used twice."));
        }
        let sex = value_of(s, "sex");
        if !SEXES.contains(&sex.as_str()) {
            errors.push(format!(r#"{place}: "sex" must be "male" or "female"."#));
        }
        if let Some(skin) = s.get("skin")
            && !skin.as_str().is_some_and(|v| SKIN_TONES.contains(&v.trim().to_lowercase().as_str()))
        {
            errors.push(format!(r#"{place}: "skin" must be one of {}."#, SKIN_TONES.join(", ")));
        }
        for k in &keys {
            if text(s.get(*k)).is_none() {
                errors.push(format!(r#"{place}: missing "{k}"."#));
            }
        }
        let mut profile: Vec<String> = searchable.iter().map(|k| value_of(s, k)).collect();
        profile.insert(0, sex);
        if let Some(other) = profiles.get(&profile) {
            errors.push(format!(
                "{place}: same sex and crime computer traits as {other}, so the computer could never tell them apart. Change a trait."
            ));
        }
        profiles.insert(profile, name);
    }
    for sex in SEXES {
        let alone: Vec<&str> =
            suspects.iter().filter(|s| value_of(s, "sex") == sex).filter_map(|s| s.get("name").and_then(Value::as_str)).collect();
        if let [only] = alone[..] {
            warnings.push(format!(
                r#"suspects.json: {only} is the only {sex}, so a witness saying "{}" gives them away."#,
                if sex == "female" { "she" } else { "he" }
            ));
        }
    }
    for k in &searchable {
        let values: HashSet<String> = suspects.iter().filter(|s| s.is_object()).map(|s| value_of(s, k)).collect();
        if suspects.len() > 2 && values.len() == 1 {
            warnings.push(format!(r#"suspects.json: everyone has the same "{k}", so that clue never helps."#));
        }
    }
    (errors, warnings)
}

/// Turn validated JSON into typed data. Only call after `validate` found no errors.
fn build(cdoc: &Value, sdoc: &Value) -> Data {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let list = |v: &Value| v.as_array().map(|a| a.iter().map(s).collect()).unwrap_or_default();
    let optional = |v: Option<&Value>| text(v).map(str::to_string);

    let countries = cdoc["countries"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| Country {
            country: s(&c["country"]),
            city: s(&c["city"]),
            lat: c["lat"].as_f64().unwrap_or_default(),
            lon: c["lon"].as_f64().unwrap_or_default(),
            about: optional(c.get("about")),
            treasures: list(&c["treasures"]),
            clues: c["clues"].as_object().into_iter().flatten().map(|(cat, texts)| (cat.clone(), list(texts))).collect(),
        })
        .collect();
    let traits: Vec<Trait> = sdoc["traits"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| Trait {
            key: s(&t["key"]),
            label: s(&t["label"]),
            clues: list(&t["clues"]),
            computer: t.get("computer").and_then(Value::as_bool).unwrap_or(true),
            phrases: t
                .get("phrases")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(k, v)| (k.trim().to_lowercase(), s(v)))
                .collect(),
        })
        .collect();
    let suspects = sdoc["suspects"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| Suspect {
            name: s(&p["name"]),
            sex: s(&p["sex"]).trim().to_lowercase(),
            traits: traits.iter().map(|t| (t.key.clone(), s(&p[&t.key]).trim().to_string())).collect(),
            bio: optional(p.get("bio")),
            catchphrase: optional(p.get("catchphrase")),
            skin: optional(p.get("skin")).map(|s| s.trim().to_lowercase()),
        })
        .collect();
    Data { countries, traits, suspects }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_data_is_valid() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(load(Some(dir)).is_ok(), "the data files have errors; run cargo run -- --check");
    }

    #[test]
    fn checks_skin_tones() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cdoc: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("countries.json")).unwrap()).unwrap();
        let mut sdoc: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("suspects.json")).unwrap()).unwrap();
        sdoc["suspects"][0]["skin"] = Value::from("tan");
        assert!(validate(&cdoc, &sdoc).0.is_empty());
        sdoc["suspects"][0]["skin"] = Value::from("green");
        assert!(validate(&cdoc, &sdoc).0.iter().any(|e| e.contains(r#""skin" must be"#)));
    }

    #[test]
    fn finds_placeholders() {
        assert_eq!(placeholders("{They} had {value}, {{them}} and {not one"), ["They", "value", "them"]);
    }

    #[test]
    fn spots_whole_words_only() {
        assert!(names_word("They loved Peru.", "peru"));
        assert!(!names_word("They loved Peruvian food.", "Peru"));
        assert!(!names_word("The Indian Ocean", "India"));
    }
}
