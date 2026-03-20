/**
 * Waveguide sample loader — pre-convolves excitation × body IR
 * using OfflineAudioContext + ConvolverNode (FFT-based, exact).
 * Caches results and uploads to WASM via postMessage.
 */

const EXCITATION_FILES = [
  '/samples/excitations/anvil-strike.wav',
  '/samples/excitations/air-hiss-01.wav',
  '/samples/excitations/bubble-pop.wav',
  '/samples/excitations/door-stop-twang-01.wav',
  '/samples/excitations/hammer-anvil-hit-01.wav',
  '/samples/excitations/relay-click-01.wav',
];

const BODY_FILES = [
  '/samples/bodies/glass-resonance-01.wav',
  '/samples/bodies/metal-bar-resonance-01.wav',
  '/samples/bodies/metal-tube-clear-01.wav',
  '/samples/bodies/tubular-bell-strike-01.wav',
  '/samples/bodies/wine-glass-ring-01.wav',
];

const MAX_WAVETABLE_LEN = 16384;
const FADE_SAMPLES = 512;
const NORMALIZE_PEAK = 0.5;

async function fetchAudioBuffer(ctx: AudioContext, url: string): Promise<AudioBuffer> {
  const response = await fetch(url);
  const arrayBuf = await response.arrayBuffer();
  return ctx.decodeAudioData(arrayBuf);
}

async function preConvolve(
  excitation: AudioBuffer,
  bodyIR: AudioBuffer
): Promise<Float32Array> {
  const fullLength = excitation.length + bodyIR.length - 1;
  const sr = excitation.sampleRate;

  const offline = new OfflineAudioContext(1, fullLength, sr);
  const source = offline.createBufferSource();
  source.buffer = excitation;
  const convolver = new ConvolverNode(offline, { buffer: bodyIR });
  source.connect(convolver);
  convolver.connect(offline.destination);
  source.start();

  const rendered = await offline.startRendering();

  // Truncate to max length
  const fullData = rendered.getChannelData(0);
  const len = Math.min(fullData.length, MAX_WAVETABLE_LEN);
  const data = new Float32Array(len);
  data.set(fullData.subarray(0, len));

  // Cosine fade-out at truncation boundary
  const fadeLen = Math.min(FADE_SAMPLES, len);
  for (let i = 0; i < fadeLen; i++) {
    const t = i / fadeLen;
    data[len - fadeLen + i] *= 0.5 * (1 + Math.cos(Math.PI * t));
  }

  // Normalize to 0.5 peak
  let peak = 0;
  for (let i = 0; i < len; i++) {
    const abs = Math.abs(data[i]);
    if (abs > peak) peak = abs;
  }
  if (peak > 0.001) {
    const scale = NORMALIZE_PEAK / peak;
    for (let i = 0; i < len; i++) {
      data[i] *= scale;
    }
  }

  return data;
}

export interface WaveguidePresets {
  excitations: AudioBuffer[];
  bodies: AudioBuffer[];
  cache: Map<string, Float32Array>;
}

/**
 * Load all excitation and body samples, return a loader that can
 * pre-convolve any combination on demand.
 */
export async function loadWaveguideSamples(): Promise<WaveguidePresets> {
  const ctx = new AudioContext();

  const [excitations, bodies] = await Promise.all([
    Promise.all(EXCITATION_FILES.map(f => fetchAudioBuffer(ctx, f))),
    Promise.all(BODY_FILES.map(f => fetchAudioBuffer(ctx, f))),
  ]);

  await ctx.close();

  return {
    excitations,
    bodies,
    cache: new Map(),
  };
}

/**
 * Get a pre-convolved wavetable for a specific excitation × body combination.
 * Cached after first computation.
 */
export async function getConvolvedWavetable(
  presets: WaveguidePresets,
  excitationIdx: number,
  bodyIdx: number
): Promise<Float32Array> {
  const key = `${excitationIdx}-${bodyIdx}`;
  const cached = presets.cache.get(key);
  if (cached) return cached;

  const exc = presets.excitations[excitationIdx];
  const body = presets.bodies[bodyIdx];
  if (!exc || !body) {
    return new Float32Array(0);
  }

  const result = await preConvolve(exc, body);
  presets.cache.set(key, result);
  return result;
}

export const EXCITATION_COUNT = EXCITATION_FILES.length;
export const BODY_COUNT = BODY_FILES.length;
