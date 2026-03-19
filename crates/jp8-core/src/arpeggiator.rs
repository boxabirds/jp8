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

    pub fn get_held_count(&self) -> usize {
        self.held_count
    }

    pub fn get_sequence_len(&self) -> usize {
        self.sequence_len
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

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    fn make_arp(mode: ArpMode) -> Arpeggiator {
        let mut arp = Arpeggiator::new(SR);
        arp.mode = mode;
        arp
    }

    /// Tick the arpeggiator until it fires a note_on, returning that note.
    /// Returns 0 if nothing fires within `max_ticks`.
    fn tick_until_note(arp: &mut Arpeggiator, max_ticks: usize) -> u8 {
        for _ in 0..max_ticks {
            let (on, _) = arp.tick();
            if on > 0 { return on; }
        }
        0
    }

    #[test]
    fn off_mode_passthrough() {
        let mut arp = make_arp(ArpMode::Off);
        assert!(!arp.note_on(60));
        assert!(!arp.note_off(60));
    }

    #[test]
    fn up_ascending() {
        let mut arp = make_arp(ArpMode::Up);
        arp.set_tempo(120.0);
        arp.note_on(60); // C4
        arp.note_on(64); // E4
        arp.note_on(67); // G4

        let mut seq = Vec::new();
        let step_samples = (SR * 60.0 / 120.0 / 4.0) as usize;
        for _ in 0..6 {
            let note = tick_until_note(&mut arp, step_samples + 100);
            if note > 0 { seq.push(note); }
        }
        // Should cycle: C4, E4, G4, C4, E4, G4
        assert!(seq.len() >= 3, "Should produce at least 3 notes, got {:?}", seq);
        assert_eq!(seq[0], 60);
        assert_eq!(seq[1], 64);
        assert_eq!(seq[2], 67);
    }

    #[test]
    fn down_descending() {
        let mut arp = make_arp(ArpMode::Down);
        arp.set_tempo(120.0);
        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        let step_samples = (SR * 60.0 / 120.0 / 4.0) as usize;
        let mut seq = Vec::new();
        for _ in 0..6 {
            let note = tick_until_note(&mut arp, step_samples + 100);
            if note > 0 { seq.push(note); }
        }
        assert!(seq.len() >= 3);
        assert_eq!(seq[0], 67);
        assert_eq!(seq[1], 64);
        assert_eq!(seq[2], 60);
    }

    #[test]
    fn updown_bounces() {
        let mut arp = make_arp(ArpMode::UpDown);
        arp.set_tempo(120.0);
        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        let step_samples = (SR * 60.0 / 120.0 / 4.0) as usize;
        let mut seq = Vec::new();
        for _ in 0..9 {
            let note = tick_until_note(&mut arp, step_samples + 100);
            if note > 0 { seq.push(note); }
        }
        // Up phase: 60, 64, 67, then Down phase: 67, 64, 60
        assert!(seq.len() >= 6, "Expected 6+ notes, got {:?}", seq);
        // First 3 ascending
        assert_eq!(seq[0], 60);
        assert_eq!(seq[1], 64);
        assert_eq!(seq[2], 67);
        // Next 3 descending
        assert_eq!(seq[3], 67);
        assert_eq!(seq[4], 64);
        assert_eq!(seq[5], 60);
    }

    #[test]
    fn range_2_octaves() {
        let mut arp = make_arp(ArpMode::Up);
        arp.range_octaves = 2;
        arp.set_tempo(120.0);
        arp.note_on(60); // C4

        let step_samples = (SR * 60.0 / 120.0 / 4.0) as usize;
        let mut seq = Vec::new();
        for _ in 0..4 {
            let note = tick_until_note(&mut arp, step_samples + 100);
            if note > 0 { seq.push(note); }
        }
        assert!(seq.len() >= 2);
        assert_eq!(seq[0], 60);  // C4
        assert_eq!(seq[1], 72);  // C5 (octave up)
    }

    #[test]
    fn tempo_step_rate() {
        let arp = make_arp(ArpMode::Up);
        // At 120 BPM, 16th note = SR * 60 / 120 / 4
        let expected = SR * 60.0 / 120.0 / 4.0;
        assert!((arp.samples_per_step - expected).abs() < 1.0,
            "Expected ~{expected}, got {}", arp.samples_per_step);
    }

    #[test]
    fn add_note_sorted() {
        let mut arp = make_arp(ArpMode::Up);
        arp.note_on(67); // G
        arp.note_on(60); // C
        arp.note_on(64); // E
        assert_eq!(arp.held[0], 60);
        assert_eq!(arp.held[1], 64);
        assert_eq!(arp.held[2], 67);
    }

    #[test]
    fn remove_note_shifts() {
        let mut arp = make_arp(ArpMode::Up);
        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);
        arp.note_off(64); // remove middle
        assert_eq!(arp.get_held_count(), 2);
        assert_eq!(arp.held[0], 60);
        assert_eq!(arp.held[1], 67);
    }

    #[test]
    fn duplicate_ignored() {
        let mut arp = make_arp(ArpMode::Up);
        arp.note_on(60);
        arp.note_on(60);
        assert_eq!(arp.get_held_count(), 1);
    }

    #[test]
    fn max_16_held_notes() {
        let mut arp = make_arp(ArpMode::Up);
        for note in 40..56 { // 16 notes
            arp.note_on(note);
        }
        assert_eq!(arp.get_held_count(), 16);
        arp.note_on(57); // 17th — should be rejected
        assert_eq!(arp.get_held_count(), 16);
    }

    #[test]
    fn all_off_resets() {
        let mut arp = make_arp(ArpMode::Up);
        arp.note_on(60);
        arp.note_on(64);
        arp.all_off();
        assert_eq!(arp.get_held_count(), 0);
        assert_eq!(arp.get_sequence_len(), 0);
        assert_eq!(arp.current_note, 0);
    }

    #[test]
    fn is_active_requires_notes_and_mode() {
        let mut arp = make_arp(ArpMode::Off);
        arp.note_on(60); // won't add because mode is Off
        assert!(!arp.is_active());

        let mut arp = make_arp(ArpMode::Up);
        assert!(!arp.is_active()); // no notes
        arp.note_on(60);
        assert!(arp.is_active()); // mode + notes
    }

    #[test]
    fn tick_inactive_returns_zero() {
        let mut arp = make_arp(ArpMode::Up);
        // No notes held
        let (on, off) = arp.tick();
        assert_eq!(on, 0);
        assert_eq!(off, 0);
    }

    #[test]
    fn note_saturating_add() {
        let mut arp = make_arp(ArpMode::Up);
        arp.range_octaves = 4;
        arp.set_tempo(300.0); // fast
        arp.note_on(120); // near MIDI max

        let step_samples = (SR * 60.0 / 300.0 / 4.0) as usize;
        for _ in 0..8 {
            let note = tick_until_note(&mut arp, step_samples + 100);
            assert!(note <= 127, "Note exceeded MIDI max: {note}");
        }
    }
}
