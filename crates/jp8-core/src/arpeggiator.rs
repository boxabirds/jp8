/// JP-8 arpeggiator.
/// Modes: Up, Down, Up+Down. Range: 1–4 octaves.
/// Runs sample-accurate on the audio thread.

const MAX_HELD_NOTES: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum ArpMode {
    Off,
    Up,
    Down,
    UpDown,
}

pub struct Arpeggiator {
    pub mode: ArpMode,
    pub range_octaves: u8,  // 1–4
    pub tempo_bpm: f32,     // 30–300

    // Held notes (sorted)
    held: [u8; MAX_HELD_NOTES],
    held_count: usize,

    // Playback state
    sequence_pos: usize,
    sequence_len: usize,
    going_up: bool,         // for UpDown mode direction
    samples_per_step: f32,
    sample_counter: f32,
    current_note: u8,       // currently sounding arp note (0 = none)
    sample_rate: f32,
}

impl Arpeggiator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            mode: ArpMode::Off,
            range_octaves: 1,
            tempo_bpm: 120.0,
            held: [0; MAX_HELD_NOTES],
            held_count: 0,
            sequence_pos: 0,
            sequence_len: 0,
            going_up: true,
            samples_per_step: sample_rate * 60.0 / 120.0 / 4.0, // 16th notes at 120 BPM
            sample_counter: 0.0,
            current_note: 0,
            sample_rate,
        }
    }

    pub fn set_tempo(&mut self, bpm: f32) {
        self.tempo_bpm = bpm.clamp(30.0, 300.0);
        // 16th note duration
        self.samples_per_step = self.sample_rate * 60.0 / self.tempo_bpm / 4.0;
    }

    /// Called when user presses a key. Returns true if arp should handle this note.
    pub fn note_on(&mut self, note: u8) -> bool {
        if self.mode == ArpMode::Off {
            return false;
        }

        // Add to held notes (sorted, no duplicates)
        if self.held_count < MAX_HELD_NOTES && !self.held[..self.held_count].contains(&note) {
            self.held[self.held_count] = note;
            self.held_count += 1;
            // Sort
            self.held[..self.held_count].sort_unstable();
            self.rebuild_sequence();
        }
        true
    }

    /// Called when user releases a key. Returns true if arp handled it.
    pub fn note_off(&mut self, note: u8) -> bool {
        if self.mode == ArpMode::Off {
            return false;
        }

        // Remove from held
        if let Some(pos) = self.held[..self.held_count].iter().position(|&n| n == note) {
            for i in pos..self.held_count - 1 {
                self.held[i] = self.held[i + 1];
            }
            self.held_count -= 1;
            self.rebuild_sequence();
        }
        true
    }

    /// Tick once per sample. Returns (note_on, note_off) events if a step boundary is crossed.
    /// note values are 0 for "no event".
    #[inline(always)]
    pub fn tick(&mut self) -> (u8, u8) {
        if self.mode == ArpMode::Off || self.sequence_len == 0 {
            let off = self.current_note;
            self.current_note = 0;
            return (0, off);
        }

        self.sample_counter += 1.0;
        if self.sample_counter < self.samples_per_step {
            return (0, 0);
        }
        self.sample_counter -= self.samples_per_step;

        let note_off = self.current_note;

        // Get next note in sequence
        let base_note = self.next_in_sequence();
        self.current_note = base_note;

        (base_note, note_off)
    }

    pub fn all_off(&mut self) {
        self.held_count = 0;
        self.sequence_len = 0;
        self.sequence_pos = 0;
        self.current_note = 0;
    }

    pub fn is_active(&self) -> bool {
        self.mode != ArpMode::Off && self.held_count > 0
    }

    fn rebuild_sequence(&mut self) {
        // Sequence length = held_count × range_octaves
        // For UpDown, we don't double — handled by direction toggle
        self.sequence_len = self.held_count * self.range_octaves as usize;
        if self.sequence_pos >= self.sequence_len {
            self.sequence_pos = 0;
        }
    }

    fn next_in_sequence(&mut self) -> u8 {
        if self.held_count == 0 {
            return 0;
        }

        let total = self.sequence_len;
        if total == 0 {
            return 0;
        }

        // Map sequence_pos to (note_index, octave)
        let note_idx = self.sequence_pos % self.held_count;
        let octave = (self.sequence_pos / self.held_count) as u8;

        let base = match self.mode {
            ArpMode::Up => self.held[note_idx],
            ArpMode::Down => {
                let rev_oct = (self.range_octaves - 1) - octave;
                let rev_idx = self.held_count - 1 - note_idx;
                self.held[rev_idx].saturating_add(rev_oct * 12)
            }
            ArpMode::UpDown => {
                if self.going_up {
                    self.held[note_idx]
                } else {
                    let rev_oct = (self.range_octaves - 1) - octave;
                    let rev_idx = self.held_count - 1 - note_idx;
                    self.held[rev_idx].saturating_add(rev_oct * 12)
                }
            }
            ArpMode::Off => 0,
        };

        let note = if self.mode == ArpMode::Down {
            base // already computed with octave
        } else if self.mode == ArpMode::UpDown && !self.going_up {
            base
        } else {
            base.saturating_add(octave * 12)
        };

        // Advance
        self.sequence_pos += 1;
        if self.sequence_pos >= total {
            self.sequence_pos = 0;
            if self.mode == ArpMode::UpDown {
                self.going_up = !self.going_up;
            }
        }

        note.min(127)
    }
}
