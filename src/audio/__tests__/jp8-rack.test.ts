import { describe, test, expect, vi, beforeEach } from 'vitest';
import { JP8Rack } from '../jp8-rack';

// Mock JP8Engine — must use function (not arrow) for `new`
vi.mock('../jp8-engine', () => ({
  JP8Engine: vi.fn(function (this: any) {
    this.start = vi.fn().mockResolvedValue(undefined);
    this.stop = vi.fn().mockResolvedValue(undefined);
    this.getAudioNode = vi.fn().mockReturnValue(null);
    this.noteOn = vi.fn();
    this.noteOff = vi.fn();
    this.allNotesOff = vi.fn();
    this.setParam = vi.fn();
    this.getStatus = vi.fn().mockReturnValue('idle');
    this.setStatusCallback = vi.fn();
  }),
}));

// Mock AudioContext — must use function for `new`
vi.stubGlobal('AudioContext', vi.fn(function (this: any) {
  this.sampleRate = 44100;
  this.destination = {};
  this.createGain = vi.fn(() => ({
    gain: { value: 1.0 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  }));
  this.createStereoPanner = vi.fn(() => ({
    pan: { value: 0 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  }));
  this.resume = vi.fn().mockResolvedValue(undefined);
  this.close = vi.fn().mockResolvedValue(undefined);
}));

describe('JP8Rack', () => {
  let rack: JP8Rack;

  beforeEach(() => {
    rack = new JP8Rack();
  });

  test('constructor defaults', () => {
    expect(rack.getStatus()).toBe('idle');
    expect(rack.getInstances()).toHaveLength(0);
    expect(rack.getActiveId()).toBe(0);
  });

  test('addInstance creates instance with default channel config', async () => {
    const inst = await rack.addInstance('Test');
    expect(inst.name).toBe('Test');
    expect(inst.channel.volume).toBe(0.7);
    expect(inst.channel.pan).toBe(0);
    expect(inst.channel.mute).toBe(false);
    expect(inst.channel.solo).toBe(false);
    expect(inst.channel.midiChannel).toBe(0);
  });

  test('first instance becomes activeId', async () => {
    const inst = await rack.addInstance();
    expect(rack.getActiveId()).toBe(inst.id);
  });

  test('max instance limit throws', async () => {
    for (let i = 0; i < 8; i++) {
      await rack.addInstance();
    }
    await expect(rack.addInstance()).rejects.toThrow(/Maximum 8/);
  });

  test('removeInstance cleans up and moves activeId', async () => {
    await rack.start();
    const inst1 = await rack.addInstance();
    const inst2 = await rack.addInstance();
    rack.setActiveId(inst1.id);

    await rack.removeInstance(inst1.id);
    expect(rack.getInstances()).toHaveLength(1);
    expect(rack.getActiveId()).toBe(inst2.id);
  });

  test('setActiveId rejects unknown IDs', async () => {
    const inst = await rack.addInstance();
    rack.setActiveId(999);
    expect(rack.getActiveId()).toBe(inst.id);
  });

  test('setChannelVolume updates config', async () => {
    const inst = await rack.addInstance();
    rack.setChannelVolume(inst.id, 0.3);
    expect(rack.getInstances()[0].channel.volume).toBe(0.3);
  });

  test('setChannelMute updates config', async () => {
    const inst = await rack.addInstance();
    rack.setChannelMute(inst.id, true);
    expect(rack.getInstances()[0].channel.mute).toBe(true);
  });

  test('setChannelSolo updates config', async () => {
    const inst = await rack.addInstance();
    rack.setChannelSolo(inst.id, true);
    expect(rack.getInstances()[0].channel.solo).toBe(true);
  });

  test('setChannelMidiChannel updates config', async () => {
    const inst = await rack.addInstance();
    rack.setChannelMidiChannel(inst.id, 3);
    expect(rack.getInstances()[0].channel.midiChannel).toBe(3);
  });

  describe('MIDI routing', () => {
    test('OMNI channel routes to active instance only', async () => {
      await rack.start();
      const inst1 = await rack.addInstance();
      const inst2 = await rack.addInstance();
      rack.setActiveId(inst1.id);

      rack.routeNoteOn(0, 60, 100);

      expect(inst1.engine.noteOn).toHaveBeenCalledWith(60, 100);
      expect(inst2.engine.noteOn).not.toHaveBeenCalled();
    });

    test('specific channel routes to matching instance', async () => {
      await rack.start();
      const inst1 = await rack.addInstance();
      const inst2 = await rack.addInstance();
      rack.setChannelMidiChannel(inst2.id, 3);

      rack.routeNoteOn(2, 60, 100); // incoming channel 2 → matches midiChannel 3 (2+1)

      expect(inst2.engine.noteOn).toHaveBeenCalledWith(60, 100);
    });

    test('routeCC maps CC values correctly', async () => {
      await rack.start();
      const inst = await rack.addInstance();
      rack.setActiveId(inst.id);

      // CC#74 (cutoff) → param 12, mapped to Hz
      rack.routeCC(0, 74, 127);
      expect(inst.engine.setParam).toHaveBeenCalledWith(12, 20 + (127 / 127) * 19980);

      // CC#1 (mod wheel) → param 28, normalized
      rack.routeCC(0, 1, 64);
      expect(inst.engine.setParam).toHaveBeenCalledWith(28, 64 / 127);

      // CC#123 (all notes off)
      rack.routeCC(0, 123, 0);
      expect(inst.engine.allNotesOff).toHaveBeenCalled();
    });
  });
});
