//! El módulo `audio` genera la música y los efectos de sonido del juego por síntesis,
//! sin archivos externos: un chiptune en La menor y efectos cortos estilo 8 bits.

use rodio::buffer::SamplesBuffer;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, Source};
use std::num::NonZero;
use std::time::Duration;

/// Frecuencia de muestreo de todo el audio generado.
const SAMPLE_RATE: u32 = 44_100;
/// Duración de una semicorchea a 132 BPM, en segundos.
const STEP: f64 = 60.0 / 132.0 / 4.0;
/// Pasos (semicorcheas) por compás.
const STEPS_PER_BAR: u64 = 16;
/// Volumen general de la música.
const MUSIC_VOLUME: f32 = 0.55;
/// Volumen general de los efectos de sonido.
const SFX_VOLUME: f32 = 0.8;

/// Progresión Am – F – C – G: nota MIDI de la fundamental y si el acorde es menor.
const PROGRESSION: [(u8, bool); 4] = [(45, true), (41, false), (48, false), (43, false)];

/// Melodía de 4 compases: nota MIDI que empieza en ese paso, 0 = sostener la anterior, -1 = silencio.
const MELODY: [i8; 64] = [
    76, 0, 0, 0, 81, 0, 79, 0, 76, 0, 0, 0, 74, 0, 76, 0,
    72, 0, 0, 0, 77, 0, 76, 0, 72, 0, 0, 0, 69, 0, 72, 0,
    79, 0, 0, 0, 76, 0, 79, 0, 84, 0, 0, 0, 83, 0, 79, 0,
    74, 0, 0, 0, 79, 0, 77, 0, 76, 0, 74, 0, 71, 0, -1, 0,
];

/// Efectos de sonido disponibles.
#[derive(Clone, Copy)]
pub enum Sfx {
    /// Al recoger un ítem.
    Pickup,
    /// Al recibir daño de un enemigo.
    Hurt,
    /// Al perder todas las vidas.
    GameOver,
    /// Al completar un nivel.
    LevelComplete,
}

/// Reproductor de audio del juego. Si no hay dispositivo de salida, el juego sigue en silencio.
pub struct Audio {
    _device: MixerDeviceSink,
    music: Player,
    sfx: Player,
    muted: bool,
}

impl Audio {
    /// Abre el dispositivo de audio por defecto y empieza la música. Devuelve `None` si no hay audio.
    pub fn new() -> Option<Self> {
        let mut device = DeviceSinkBuilder::open_default_sink().ok()?;
        device.log_on_drop(false);

        let music = Player::connect_new(device.mixer());
        music.set_volume(MUSIC_VOLUME);
        music.append(Music::new());

        let sfx = Player::connect_new(device.mixer());
        sfx.set_volume(SFX_VOLUME);

        Some(Self { _device: device, music, sfx, muted: false })
    }

    /// Reproduce un efecto de sonido. Si hay otro sonando, lo reemplaza para responder al instante.
    pub fn play(&self, effect: Sfx) {
        self.sfx.clear();
        self.sfx.append(render_sfx(effect));
        self.sfx.play();
    }

    /// Silencia o reactiva todo el audio.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let (music, sfx) = if self.muted { (0.0, 0.0) } else { (MUSIC_VOLUME, SFX_VOLUME) };
        self.music.set_volume(music);
        self.sfx.set_volume(sfx);
    }
}

/// Fuente de audio infinita que sintetiza la música muestra por muestra.
pub struct Music {
    index: u64,
    noise: u32,
}

impl Music {
    pub fn new() -> Self {
        Self { index: 0, noise: 0x1234_5678 }
    }

    /// Ruido blanco en [-1, 1] (xorshift).
    fn next_noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    fn sample(&mut self) -> f32 {
        let t = self.index as f64 / SAMPLE_RATE as f64;
        let step = (t / STEP) as u64;
        let since_step = (t - step as f64 * STEP) as f32;
        let bar = (step / STEPS_PER_BAR) % 8;
        let beat = step % STEPS_PER_BAR;
        let (root, minor) = PROGRESSION[(bar % 4) as usize];
        let third = if minor { 3 } else { 4 };

        // Bajo: corcheas sobre la fundamental, con salto de octava en el tercer tiempo.
        let bass_note = if (beat / 2) % 4 == 2 { root + 12 } else { root };
        let bass_t = (t - (step - step % 2) as f64 * STEP) as f32;
        let bass = triangle(midi_freq(bass_note) * bass_t) * envelope(bass_t, 0.25) * 0.22;

        // Arpegio: una nota del acorde por semicorchea.
        let chord = [root + 24, root + 24 + third, root + 31];
        let arp_note = chord[(step % 3) as usize];
        let arp = square(midi_freq(arp_note) * since_step, 0.25) * envelope(since_step, 0.07) * 0.05;

        // Melodía: solo en la segunda mitad del ciclo de 8 compases, para dar variedad.
        let mut lead = 0.0;
        if bar >= 4 {
            let base = ((bar % 4) * STEPS_PER_BAR) as usize;
            let mut start = beat as usize;
            while start > 0 && MELODY[base + start] == 0 {
                start -= 1;
            }
            let note = MELODY[base + start];
            if note > 0 {
                let lead_t = since_step + (beat as usize - start) as f32 * STEP as f32;
                let vibrato = 1.0 + 0.004 * (lead_t * 30.0).sin();
                lead = square(midi_freq(note as u8) * lead_t * vibrato, 0.5) * envelope(lead_t, 0.4) * 0.07;
            }
        }

        // Batería: bombo, caja y charles.
        let mut drums = 0.0;
        let kick = beat == 0 || beat == 8 || (beat == 10 && bar % 2 == 1);
        if kick {
            let freq = 45.0 + 110.0 * (-since_step * 25.0).exp();
            drums += (std::f32::consts::TAU * freq * since_step).sin() * envelope(since_step, 0.12) * 0.35;
        }
        let noise = self.next_noise();
        if beat == 4 || beat == 12 {
            drums += noise * envelope(since_step, 0.07) * 0.12;
        }
        if beat % 2 == 1 {
            drums += noise * envelope(since_step, 0.015) * 0.04;
        }

        // Saturación suave para que la mezcla nunca recorte.
        (bass + arp + lead + drums).tanh()
    }
}

impl Default for Music {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for Music {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.sample();
        self.index += 1;
        Some(sample)
    }
}

impl Source for Music {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Genera las muestras de un efecto de sonido.
pub fn render_sfx(effect: Sfx) -> SamplesBuffer {
    let mut noise = 0x9e37_79b9u32;
    let mut next_noise = move || {
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        noise as f32 / u32::MAX as f32 * 2.0 - 1.0
    };

    let samples: Vec<f32> = match effect {
        Sfx::Pickup => notes(&[(88, 0.06), (95, 0.14)], 0.5, 0.12),
        Sfx::LevelComplete => notes(&[(69, 0.09), (72, 0.09), (76, 0.09), (81, 0.35)], 0.5, 0.2),
        Sfx::GameOver => notes(&[(69, 0.18), (65, 0.18), (62, 0.18), (57, 0.5)], 0.5, 0.3),
        Sfx::Hurt => {
            let len = (0.3 * SAMPLE_RATE as f32) as usize;
            let mut phase = 0.0;
            (0..len)
                .map(|i| {
                    let t = i as f32 / SAMPLE_RATE as f32;
                    phase += (320.0 * (-t * 6.0).exp() + 60.0) / SAMPLE_RATE as f32;
                    let tone = square(phase, 0.5) * 0.25;
                    (tone + next_noise() * 0.2) * envelope(t, 0.1)
                })
                .collect()
        }
    };

    SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(SAMPLE_RATE).unwrap(), samples)
}

/// Secuencia de notas cuadradas: (nota MIDI, duración en segundos).
fn notes(sequence: &[(u8, f32)], duty: f32, decay: f32) -> Vec<f32> {
    let mut samples = Vec::new();
    for &(note, duration) in sequence {
        let len = (duration * SAMPLE_RATE as f32) as usize;
        let freq = midi_freq(note);
        samples.extend((0..len).map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            square(freq * t, duty) * envelope(t, decay) * 0.3
        }));
    }
    samples
}

/// Frecuencia en Hz de una nota MIDI.
fn midi_freq(note: u8) -> f32 {
    440.0 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

/// Onda cuadrada a partir de la fase en ciclos, con ciclo de trabajo `duty`.
fn square(phase: f32, duty: f32) -> f32 {
    if phase.fract() < duty { 1.0 } else { -1.0 }
}

/// Onda triangular a partir de la fase en ciclos.
fn triangle(phase: f32) -> f32 {
    4.0 * (phase.fract() - 0.5).abs() - 1.0
}

/// Envolvente de ataque corto y caída exponencial.
fn envelope(t: f32, decay: f32) -> f32 {
    const ATTACK: f32 = 0.003;
    if t < ATTACK { t / ATTACK } else { (-(t - ATTACK) / decay).exp() }
}
