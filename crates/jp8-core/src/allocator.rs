/// Voice allocator with JP-8 assign modes.
/// Per the JP-8 spec §2.7.

const MAX_VOICES: usize = 8;

#[derive(Clone, Copy)]
pub enum AssignMode {
    Poly8,
    Poly4,
    Unison,
    Solo,
}

pub struct VoiceAllocator {
    voices_active: [bool; MAX_VOICES],
    voices_note: [u8; MAX_VOICES],
    voices_env_level: [f32; MAX_VOICES],
    last_assigned: usize,
    pub mode: AssignMode,
    pub unison_detune: f32,
}

impl VoiceAllocator {
    pub fn new() -> Self {
        Self {
            voices_active: [false; MAX_VOICES],
            voices_note: [0; MAX_VOICES],
            voices_env_level: [0.0; MAX_VOICES],
            last_assigned: 0,
            mode: AssignMode::Poly8,
            unison_detune: 0.1,
        }
    }

    /// Allocate a voice for a note. Returns voice index.
    pub fn note_on(&mut self, note: u8) -> usize {
        match self.mode {
            AssignMode::Poly8 => self.alloc_poly(note, MAX_VOICES),
            AssignMode::Poly4 => self.alloc_poly(note, 4),
            AssignMode::Unison | AssignMode::Solo => {
                // All voices play the same note
                for i in 0..MAX_VOICES {
                    self.voices_active[i] = true;
                    self.voices_note[i] = note;
                }
                0
            }
        }
    }

    /// Release voice(s) for a note. Returns indices of released voices.
    pub fn note_off(&mut self, note: u8, released: &mut [usize; MAX_VOICES]) -> usize {
        let mut count = 0;
        for i in 0..MAX_VOICES {
            if self.voices_active[i] && self.voices_note[i] == note {
                self.voices_active[i] = false;
                released[count] = i;
                count += 1;
            }
        }
        count
    }

    /// Update envelope levels for voice stealing decisions.
    pub fn update_env_level(&mut self, voice: usize, level: f32) {
        if voice < MAX_VOICES {
            self.voices_env_level[voice] = level;
        }
    }

    fn alloc_poly(&mut self, note: u8, max: usize) -> usize {
        // First: find a free voice
        for i in 0..max {
            let idx = (self.last_assigned + 1 + i) % max;
            if !self.voices_active[idx] {
                self.voices_active[idx] = true;
                self.voices_note[idx] = note;
                self.last_assigned = idx;
                return idx;
            }
        }
        // Steal: voice with lowest envelope level
        let mut steal_idx = 0;
        let mut min_level = f32::MAX;
        for i in 0..max {
            if self.voices_env_level[i] < min_level {
                min_level = self.voices_env_level[i];
                steal_idx = i;
            }
        }
        self.voices_active[steal_idx] = true;
        self.voices_note[steal_idx] = note;
        self.last_assigned = steal_idx;
        steal_idx
    }

    pub fn all_off(&mut self) {
        self.voices_active = [false; MAX_VOICES];
    }
}
