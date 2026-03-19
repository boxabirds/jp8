/**
 * Web MIDI input handler for JP-8.
 * Per spec §4.3.
 */

import { JP8Engine, P } from '../audio/jp8-engine';

export async function setupMIDI(engine: JP8Engine): Promise<() => void> {
  if (!navigator.requestMIDIAccess) {
    console.warn('Web MIDI not available');
    return () => {};
  }

  let access: MIDIAccess;
  try {
    access = await navigator.requestMIDIAccess();
  } catch {
    console.warn('MIDI access denied');
    return () => {};
  }

  const handler = (event: MIDIMessageEvent) => {
    const data = event.data;
    if (!data || data.length < 2) return;

    const status = data[0] & 0xF0;
    const note = data[1];
    const velocity = data.length > 2 ? data[2] : 0;

    if (status === 0x90 && velocity > 0) {
      engine.noteOn(note, velocity);
    } else if (status === 0x80 || (status === 0x90 && velocity === 0)) {
      engine.noteOff(note);
    } else if (status === 0xB0) {
      // CC mapping
      const val = velocity / 127;
      switch (note) {
        case 1:  engine.setParam(P.LFO_PITCH, val); break; // Mod wheel
        case 7:  engine.setParam(P.VOLUME, val); break;     // Volume
        case 74: engine.setParam(P.FILTER_CUTOFF, 20 + val * 19980); break; // Cutoff
        case 71: engine.setParam(P.FILTER_RESO, val); break; // Resonance
        case 123: engine.allNotesOff(); break;
      }
    }
  };

  const attachInputs = () => {
    for (const input of access.inputs.values()) {
      input.onmidimessage = handler as EventListener;
    }
  };

  attachInputs();
  access.onstatechange = () => attachInputs();

  return () => {
    for (const input of access.inputs.values()) input.onmidimessage = null;
    access.onstatechange = null;
  };
}
