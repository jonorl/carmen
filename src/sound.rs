//! Sound effects, all cut from recordings in assets/sounds/: a printer that
//! loops while the printer is printing, a key press for typing, a clock
//! tick for each hour that passes, and footsteps for walking to a witness.

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};

/// 0.7 s of a printer at work, cut from the middle of a burst so it loops.
const PRINTING: &[u8] = include_bytes!("../assets/sounds/printer.wav");
/// One key press.
const KEY: &[u8] = include_bytes!("../assets/sounds/key.wav");
/// One tick of a clock.
const TICK: &[u8] = include_bytes!("../assets/sounds/tick.wav");
/// Five footsteps over 1.86 s.
const WALKING: &[u8] = include_bytes!("../assets/sounds/walking.wav");
/// The chase before an arrest: a sting, the suspect's running feet, a jingle
/// under "There goes the suspect!", police whistles and shouts, and the
/// police running after them. 17.6 s.
const CHASE: &[u8] = include_bytes!("../assets/sounds/chase.wav");
/// The crime computer whirring over a warrant: 2.1 s.
const COMPUTING: &[u8] = include_bytes!("../assets/sounds/computing.wav");
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
    computing: Sound,
    /// Whether the printing loop is playing.
    looping: bool,
    pub on: bool,
}

impl Sounds {
    /// None if the audio device can't take the sounds; the game then runs silently.
    pub async fn load() -> Option<Sounds> {
        let load = |bytes: &'static [u8]| async move {
            read_wav(bytes)?;
            load_sound_from_bytes(bytes).await.ok()
        };
        Some(Sounds {
            printing: load(PRINTING).await?,
            key: load(KEY).await?,
            tick: load(TICK).await?,
            walking: load(WALKING).await?,
            chase: load(CHASE).await?,
            computing: load(COMPUTING).await?,
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
}

/// The samples of a 16-bit PCM WAV file (the first channel, if there are
/// several), from -1 to 1. None if it isn't one.
fn read_wav(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut pos, mut channels, mut bits) = (12, 1usize, 0u16);
    while pos + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes.get(pos + 4..pos + 8)?.try_into().ok()?) as usize;
        let body = bytes.get(pos + 8..pos + 8 + len)?;
        match bytes.get(pos..pos + 4)? {
            b"fmt " => {
                channels = u16::from_le_bytes(body.get(2..4)?.try_into().ok()?) as usize;
                bits = u16::from_le_bytes(body.get(14..16)?.try_into().ok()?);
            }
            b"data" if bits == 16 => {
                let frame = 2 * channels.max(1);
                return Some(body.chunks_exact(frame).map(|f| i16::from_le_bytes([f[0], f[1]]) as f32 / 32768.0).collect());
            }
            _ => {}
        }
        pos += 8 + len + (len & 1); // chunks are padded to an even length
    }
    None
}

#[cfg(test)]
const RATE: u32 = 44_100;

/// 16-bit mono PCM in a WAV container, for testing read_wav.
#[cfg(test)]
fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes()); // fmt chunk size
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes()); // bytes per second
    out.extend(2u16.to_le_bytes()); // bytes per frame
    out.extend(16u16.to_le_bytes()); // bits per sample
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in samples {
        out.extend(((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_the_data() {
        let bytes = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len() - 8);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn reads_the_printer_recordings() {
        for (name, bytes) in [("printer.wav", PRINTING), ("key.wav", KEY), ("tick.wav", TICK), ("walking.wav", WALKING), ("chase.wav", CHASE), ("computing.wav", COMPUTING)] {
            let samples = read_wav(bytes).unwrap_or_else(|| panic!("{name} is 16-bit PCM"));
            assert!(samples.iter().any(|s| s.abs() > 0.1), "{name} isn't silent");
        }
    }

    #[test]
    fn wav_round_trip() {
        let samples = [0.0, 0.5, -0.5, 0.25];
        let back = read_wav(&wav(&samples)).expect("our own WAV");
        assert!(samples.iter().zip(&back).all(|(a, b)| (a - b).abs() < 1e-3));
    }


}
