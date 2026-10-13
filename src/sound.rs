//! Sound effects, all cut from recordings in assets/sounds/: a printer that
//! loops while the printer is printing, a key press for typing, a clock
//! tick for each hour that passes, footsteps for walking to a witness, and
//! the soundtracks of the chase and the arrest.

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};

/// 0.7 s of a printer at work, cut from the middle of a burst so it loops.
const PRINTING: &[u8] = include_bytes!("../assets/sounds/printer.ogg");
/// One key press.
const KEY: &[u8] = include_bytes!("../assets/sounds/key.ogg");
/// One tick of a clock.
const TICK: &[u8] = include_bytes!("../assets/sounds/tick.ogg");
/// Five footsteps over 1.86 s.
const WALKING: &[u8] = include_bytes!("../assets/sounds/walking.ogg");
/// The chase before an arrest: a sting, the suspect's running feet, a jingle
/// under "There goes the suspect!", police whistles and shouts, and the
/// police running after them. 17.6 s.
const CHASE: &[u8] = include_bytes!("../assets/sounds/chase.ogg");
/// One footstep on the way to jail, 0.667 s, so that looping it keeps the
/// officer's pace.
const FOOTSTEP: &[u8] = include_bytes!("../assets/sounds/footstep.ogg");
/// The music over the jail when the suspect is caught: 14.2 s, ending just
/// before the clank.
const CAPTURED: &[u8] = include_bytes!("../assets/sounds/captured.ogg");
/// The clank as the prisoner grabs the bars, after the music: 0.55 s.
const CLANK: &[u8] = include_bytes!("../assets/sounds/clank.ogg");
/// The sad tune after the chase when the suspect got away: 6.09 s, cut
/// where the officer's first footstep breaks it off.
const DISAPPOINT: &[u8] = include_bytes!("../assets/sounds/disappoint.ogg");
/// The V.I.L.E. henchmen's tunes, one each: the one striding across (4.0 s),
/// the one sneaking past with a sack (3.2 s) and the one popping up (4.4 s).
const HENCHMEN: [&[u8]; 3] = [
    include_bytes!("../assets/sounds/henchman-stride.ogg"),
    include_bytes!("../assets/sounds/henchman-sneak.ogg"),
    include_bytes!("../assets/sounds/henchman-popup.ogg"),
];
/// The final city's warnings, each its weapon's sound and then a short tune:
/// an axe spinning in and thudding home (4.9 s), a knife thrown and twanging
/// (4.1 s), a pistol shot (4.4 s).
const DANGERS: [&[u8]; 3] = [
    include_bytes!("../assets/sounds/danger-axe.ogg"),
    include_bytes!("../assets/sounds/danger-knife.ogg"),
    include_bytes!("../assets/sounds/danger-gun.ogg"),
];
/// Music for travelling, 5 s, looped from take-off until landing.
const TRAVELLING: &[u8] = include_bytes!("../assets/sounds/travelling.ogg");
/// The crime computer whirring over a warrant: 2.1 s.
const COMPUTING: &[u8] = include_bytes!("../assets/sounds/computing.ogg");
/// How long the computing recording lasts; the lights flash for as long.
pub const COMPUTE_SECONDS: f64 = 2.1;
/// How long the walking recording lasts, so the footprints keep pace with it.
pub const WALK_SECONDS: f64 = 1.86;

pub struct Sounds {
    printing: Sound,
    key: Sound,
    tick: Sound,
    walking: Sound,
    chase: Sound,
    footstep: Sound,
    captured: Sound,
    clank: Sound,
    disappoint: Sound,
    computing: Sound,
    travelling: Sound,
    henchmen: Vec<Sound>,
    dangers: Vec<Sound>,
    /// Whether the travelling music is playing.
    flying: bool,
    /// Whether the printing loop is playing.
    looping: bool,
    pub on: bool,
}

impl Sounds {
    /// None if the audio device can't take the sounds; the game then runs silently.
    pub async fn load() -> Option<Sounds> {
        let load = |bytes: &'static [u8]| async move {
            is_ogg_vorbis(bytes).then_some(())?;
            load_sound_from_bytes(bytes).await.ok()
        };
        Some(Sounds {
            printing: load(PRINTING).await?,
            key: load(KEY).await?,
            tick: load(TICK).await?,
            walking: load(WALKING).await?,
            chase: load(CHASE).await?,
            footstep: load(FOOTSTEP).await?,
            captured: load(CAPTURED).await?,
            clank: load(CLANK).await?,
            disappoint: load(DISAPPOINT).await?,
            computing: load(COMPUTING).await?,
            travelling: load(TRAVELLING).await?,
            henchmen: vec![load(HENCHMEN[0]).await?, load(HENCHMEN[1]).await?, load(HENCHMEN[2]).await?],
            dangers: vec![load(DANGERS[0]).await?, load(DANGERS[1]).await?, load(DANGERS[2]).await?],
            flying: false,
            looping: false,
            on: true,
        })
    }

    fn once(&self, sound: &Sound, volume: f32) {
        if self.on {
            play_sound(sound, PlaySoundParams { looped: false, volume });
        }
    }

    /// Keep the printing loop running while `busy`, and stop it otherwise.
    /// Called every frame, so turning the sound off also stops it.
    pub fn printing(&mut self, busy: bool) {
        let want = busy && self.on;
        if want && !self.looping {
            play_sound(&self.printing, PlaySoundParams { looped: true, volume: 0.6 });
        } else if !want && self.looping {
            stop_sound(&self.printing);
        }
        self.looping = want;
    }

    /// Keep the travelling music looping while `travelling`, and stop it
    /// otherwise. Called every frame of a case, like `printing`.
    pub fn travelling(&mut self, travelling: bool) {
        let want = travelling && self.on;
        if want && !self.flying {
            play_sound(&self.travelling, PlaySoundParams { looped: true, volume: 0.6 });
        } else if !want && self.flying {
            stop_sound(&self.travelling);
        }
        self.flying = want;
    }

    /// A key pressed at the keyboard.
    pub fn click(&mut self) {
        self.once(&self.key, 0.7);
    }

    /// An hour going by on the clock.
    pub fn tick(&mut self) {
        self.once(&self.tick, 0.5);
    }

    pub fn walk(&mut self) {
        self.once(&self.walking, 0.7);
    }

    pub fn stop_walking(&mut self) {
        stop_sound(&self.walking);
    }

    pub fn compute(&mut self) {
        self.once(&self.computing, 0.7);
    }

    pub fn chase(&mut self) {
        self.once(&self.chase, 0.7);
    }

    pub fn stop_chase(&mut self) {
        stop_sound(&self.chase);
    }

    /// Leaving a case: cut off whatever scene was playing.
    pub fn hush(&mut self) {
        for sound in
            [&self.walking, &self.chase, &self.footstep, &self.captured, &self.clank, &self.disappoint, &self.computing, &self.travelling]
        {
            stop_sound(sound);
        }
        self.quiet();
        self.flying = false;
    }

    fn looping(&self, sound: &Sound) {
        if self.on {
            play_sound(sound, PlaySoundParams { looped: true, volume: 0.7 });
        }
    }

    /// Footsteps on the way to jail, over and over until `quiet`.
    pub fn footsteps(&mut self) {
        self.looping(&self.footstep);
    }

    /// The sad tune when the suspect got away.
    pub fn lament(&mut self) {
        self.once(&self.disappoint, 0.7);
    }

    /// The music over the jail when the suspect is caught...
    pub fn triumph(&mut self) {
        self.once(&self.captured, 0.7);
    }

    /// ...and the clank after it.
    pub fn clank(&mut self) {
        self.once(&self.clank, 0.7);
    }

    /// Stop the footsteps and any music still playing, when the walk is over
    /// or a scene is skipped.
    pub fn quiet(&mut self) {
        for sound in [&self.footstep, &self.captured, &self.disappoint].into_iter().chain(&self.henchmen).chain(&self.dangers) {
            stop_sound(sound);
        }
    }

    /// A henchman's tune: 0 striding, 1 sneaking, 2 popping up.
    pub fn henchman(&mut self, kind: usize) {
        self.once(&self.henchmen[kind], 0.7);
    }

    /// A warning's sound and tune: 0 the axe, 1 the knife, 2 the pistol.
    pub fn danger(&mut self, kind: usize) {
        self.once(&self.dangers[kind], 0.7);
    }
}

/// Whether `bytes` look like Ogg Vorbis, the format every sound is in: the
/// audio libraries fail on anything else.
fn is_ogg_vorbis(bytes: &[u8]) -> bool {
    bytes.starts_with(b"OggS") && bytes.get(..64).is_some_and(|head| head.windows(7).any(|w| w == b"\x01vorbis"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_ogg_vorbis() {
        let named = [
            ("printer", PRINTING),
            ("key", KEY),
            ("tick", TICK),
            ("walking", WALKING),
            ("chase", CHASE),
            ("footstep", FOOTSTEP),
            ("captured", CAPTURED),
            ("clank", CLANK),
            ("disappoint", DISAPPOINT),
            ("computing", COMPUTING),
            ("travelling", TRAVELLING),
        ];
        let all = named.into_iter().chain(HENCHMEN.map(|b| ("a henchman", b))).chain(DANGERS.map(|b| ("a warning", b)));
        for (name, bytes) in all {
            assert!(is_ogg_vorbis(bytes), "{name}.ogg should be Ogg Vorbis");
        }
        assert!(!is_ogg_vorbis(b"RIFF....WAVEfmt "));
    }
}
