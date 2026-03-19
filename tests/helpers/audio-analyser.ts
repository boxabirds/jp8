/**
 * E2E audio analysis helper.
 * Injects an AnalyserNode into the page's AudioContext via window.__jp8_rack.
 */
import { Page } from '@playwright/test';

const FFT_SIZE = 2048;
const SILENCE_THRESHOLD = 0.5;

/** Set up AnalyserNode on the page. Call once after start-audio. */
export async function injectAnalyser(page: Page): Promise<void> {
  await page.evaluate((fftSize) => {
    const rack = (window as any).__jp8_rack;
    if (!rack) throw new Error('Rack not exposed on window');

    // Access private masterCtx via bracket notation (TS private is runtime-transparent)
    const ctx = (rack as any).masterCtx as AudioContext;
    if (!ctx) throw new Error('No AudioContext');

    const analyser = ctx.createAnalyser();
    analyser.fftSize = fftSize;

    const masterGain = (rack as any).masterGain as GainNode;
    if (masterGain) {
      masterGain.connect(analyser);
    }

    (window as any).__jp8_analyser = analyser;
  }, FFT_SIZE);
}

/** Get total spectral energy (sum of FFT magnitude bins). */
export async function getAudioEnergy(page: Page): Promise<number> {
  return page.evaluate(() => {
    const analyser = (window as any).__jp8_analyser as AnalyserNode;
    if (!analyser) return 0;
    const data = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(data);
    let sum = 0;
    for (let i = 0; i < data.length; i++) sum += data[i];
    return sum;
  });
}

/** Check if audio is present (energy above threshold). */
export async function isAudioPresent(page: Page): Promise<boolean> {
  const energy = await getAudioEnergy(page);
  return energy > SILENCE_THRESHOLD;
}

/** Get the frequency bin with the highest magnitude. Returns frequency in Hz. */
export async function getPeakFrequency(page: Page): Promise<number> {
  return page.evaluate(() => {
    const analyser = (window as any).__jp8_analyser as AnalyserNode;
    if (!analyser) return 0;
    const ctx = (window as any).__jp8_rack?.masterCtx as AudioContext;
    if (!ctx) return 0;

    const data = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(data);

    let maxIdx = 0;
    let maxVal = 0;
    for (let i = 0; i < data.length; i++) {
      if (data[i] > maxVal) {
        maxVal = data[i];
        maxIdx = i;
      }
    }

    const binWidth = ctx.sampleRate / analyser.fftSize;
    return maxIdx * binWidth;
  });
}

/** Get energy only in bins above a frequency threshold. */
export async function getHighFreqEnergy(page: Page, cutoffHz: number): Promise<number> {
  return page.evaluate(({ cutoff }) => {
    const analyser = (window as any).__jp8_analyser as AnalyserNode;
    const ctx = (window as any).__jp8_rack?.masterCtx as AudioContext;
    if (!analyser || !ctx) return 0;

    const data = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(data);

    const binWidth = ctx.sampleRate / analyser.fftSize;
    const startBin = Math.floor(cutoff / binWidth);

    let sum = 0;
    for (let i = startBin; i < data.length; i++) sum += data[i];
    return sum;
  }, { cutoff: cutoffHz });
}

/** Start audio and inject analyser. Common setup for sound tests. */
export async function startAndInjectAnalyser(page: Page): Promise<void> {
  await page.goto('/');
  await page.getByTestId('start-audio').click();
  await page.waitForTimeout(1000); // longer wait for WASM init
  await injectAnalyser(page);
}

/** Play a note directly on the active engine (bypasses UI pointer events). */
export async function playNote(page: Page, note: number, velocity = 100): Promise<void> {
  await page.evaluate(({ n, v }) => {
    const rack = (window as any).__jp8_rack;
    const inst = rack?.getActiveInstance();
    if (inst) inst.engine.noteOn(n, v);
  }, { n: note, v: velocity });
}

/** Release a note on the active engine. */
export async function releaseNote(page: Page, note: number): Promise<void> {
  await page.evaluate((n) => {
    const rack = (window as any).__jp8_rack;
    const inst = rack?.getActiveInstance();
    if (inst) inst.engine.noteOff(n);
  }, note);
}
