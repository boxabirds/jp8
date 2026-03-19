import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupMIDI } from '../midi';
import type { JP8Rack } from '../../audio/jp8-rack';

function makeMockRack(): JP8Rack {
  return {
    routeNoteOn: vi.fn(),
    routeNoteOff: vi.fn(),
    routeCC: vi.fn(),
  } as unknown as JP8Rack;
}

describe('MIDI routing', () => {
  let mockRack: JP8Rack;
  let capturedHandler: ((event: any) => void) | null = null;

  const mockInput = {
    onmidimessage: null as any,
  };

  beforeEach(() => {
    mockRack = makeMockRack();
    capturedHandler = null;
    mockInput.onmidimessage = null;

    Object.defineProperty(navigator, 'requestMIDIAccess', {
      value: vi.fn().mockResolvedValue({
        inputs: new Map([['input-1', mockInput]]),
        onstatechange: null,
      }),
      writable: true,
      configurable: true,
    });
  });

  async function getHandler() {
    await setupMIDI(mockRack);
    return mockInput.onmidimessage;
  }

  test('note on (0x90, vel > 0) calls routeNoteOn', async () => {
    const handler = await getHandler();
    expect(handler).toBeTruthy();
    handler({ data: new Uint8Array([0x90, 60, 100]) });
    expect(mockRack.routeNoteOn).toHaveBeenCalledWith(0, 60, 100);
  });

  test('note off (0x80) calls routeNoteOff', async () => {
    const handler = await getHandler();
    handler({ data: new Uint8Array([0x80, 60, 0]) });
    expect(mockRack.routeNoteOff).toHaveBeenCalledWith(0, 60);
  });

  test('note on with vel=0 calls routeNoteOff', async () => {
    const handler = await getHandler();
    handler({ data: new Uint8Array([0x90, 60, 0]) });
    expect(mockRack.routeNoteOff).toHaveBeenCalledWith(0, 60);
  });

  test('CC message (0xB0) calls routeCC', async () => {
    const handler = await getHandler();
    handler({ data: new Uint8Array([0xB0, 74, 100]) });
    expect(mockRack.routeCC).toHaveBeenCalledWith(0, 74, 100);
  });

  test('short message is ignored', async () => {
    const handler = await getHandler();
    handler({ data: new Uint8Array([0x90]) });
    expect(mockRack.routeNoteOn).not.toHaveBeenCalled();
    expect(mockRack.routeNoteOff).not.toHaveBeenCalled();
  });

  test('missing MIDI API returns noop cleanup', async () => {
    Object.defineProperty(navigator, 'requestMIDIAccess', {
      value: undefined,
      writable: true,
      configurable: true,
    });
    const cleanup = await setupMIDI(mockRack);
    expect(typeof cleanup).toBe('function');
    cleanup(); // should not throw
  });
});
