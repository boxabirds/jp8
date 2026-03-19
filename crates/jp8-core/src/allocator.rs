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

    /// Mark a specific voice as active (used by unison mode).
    pub fn mark_active(&mut self, voice: usize, note: u8) {
        if voice < MAX_VOICES {
            self.voices_active[voice] = true;
            self.voices_note[voice] = note;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poly8_allocates_sequentially() {
        let mut alloc = VoiceAllocator::new();
        let mut indices: Vec<usize> = Vec::new();
        for note in 60..68 {
            indices.push(alloc.note_on(note));
        }
        indices.sort();
        indices.dedup();
        assert_eq!(indices.len(), 8, "Should allocate 8 unique voices");
    }

    #[test]
    fn poly8_steals_lowest_env() {
        let mut alloc = VoiceAllocator::new();
        // Fill all 8 voices
        for note in 60..68 {
            alloc.note_on(note);
        }
        // Set env levels — voice 3 has lowest
        for i in 0..8 {
            alloc.update_env_level(i, if i == 3 { 0.01 } else { 0.5 });
        }
        // 9th note should steal voice 3
        let stolen = alloc.note_on(80);
        assert_eq!(stolen, 3, "Should steal voice with lowest env level");
    }

    #[test]
    fn poly4_only_four_voices() {
        let mut alloc = VoiceAllocator::new();
        alloc.mode = AssignMode::Poly4;
        let mut indices: Vec<usize> = Vec::new();
        for note in 60..64 {
            indices.push(alloc.note_on(note));
        }
        for &idx in &indices {
            assert!(idx < 4, "Poly4 should only use voices 0-3, got {idx}");
        }
    }

    #[test]
    fn unison_all_voices_same_note() {
        let mut alloc = VoiceAllocator::new();
        alloc.mode = AssignMode::Unison;
        alloc.note_on(60);
        for i in 0..8 {
            assert!(alloc.voices_active[i], "Voice {i} should be active in unison");
            assert_eq!(alloc.voices_note[i], 60);
        }
    }

    #[test]
    fn solo_same_as_unison() {
        let mut alloc = VoiceAllocator::new();
        alloc.mode = AssignMode::Solo;
        alloc.note_on(72);
        for i in 0..8 {
            assert!(alloc.voices_active[i]);
            assert_eq!(alloc.voices_note[i], 72);
        }
    }

    #[test]
    fn note_off_releases_correct() {
        let mut alloc = VoiceAllocator::new();
        alloc.note_on(60);
        alloc.note_on(64);
        let mut released = [0usize; 8];
        let count = alloc.note_off(60, &mut released);
        assert_eq!(count, 1);
        assert!(!alloc.voices_active[released[0]]);
    }

    #[test]
    fn note_off_unison_releases_all() {
        let mut alloc = VoiceAllocator::new();
        alloc.mode = AssignMode::Unison;
        alloc.note_on(60);
        let mut released = [0usize; 8];
        let count = alloc.note_off(60, &mut released);
        assert_eq!(count, 8, "Unison note_off should release all 8 voices");
    }

    #[test]
    fn all_off_clears_all() {
        let mut alloc = VoiceAllocator::new();
        for note in 60..68 {
            alloc.note_on(note);
        }
        alloc.all_off();
        for i in 0..8 {
            assert!(!alloc.voices_active[i]);
        }
    }

    #[test]
    fn round_robin_wraps() {
        let mut alloc = VoiceAllocator::new();
        // Fill and release, then fill again — should wrap around
        for note in 60..68 {
            alloc.note_on(note);
        }
        alloc.all_off();
        // Second round should still allocate successfully
        for note in 70..78 {
            let idx = alloc.note_on(note);
            assert!(idx < 8);
        }
    }

    #[test]
    fn env_level_tracking() {
        let mut alloc = VoiceAllocator::new();
        alloc.update_env_level(5, 0.42);
        // Fill all voices
        for note in 60..68 {
            alloc.note_on(note);
        }
        // Voice 5 has level 0.42, others default 0.0
        // But other voices were just allocated — their env_level is still 0
        // So steal should pick one of the 0-level voices (not voice 5)
        alloc.update_env_level(5, 0.42);
        for i in 0..8 {
            if i != 5 { alloc.update_env_level(i, 0.8); }
        }
        let stolen = alloc.note_on(80);
        assert_eq!(stolen, 5, "Should steal voice with lowest env level (0.42)");
    }
}
