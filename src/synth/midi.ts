/**
 * Web MIDI routing to JP-8 Rack.
 * Routes by MIDI channel to instance configs.
 */

import type { JP8Rack } from '../audio/jp8-rack';

export async function setupMIDI(rack: JP8Rack): Promise<() => void> {
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
    const channel = data[0] & 0x0F;
    const note = data[1];
    const velocity = data.length > 2 ? data[2] : 0;

    if (status === 0x90 && velocity > 0) {
      rack.routeNoteOn(channel, note, velocity);
    } else if (status === 0x80 || (status === 0x90 && velocity === 0)) {
      rack.routeNoteOff(channel, note);
    } else if (status === 0xB0) {
      rack.routeCC(channel, note, velocity);
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
