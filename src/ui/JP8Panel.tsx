import { useState, useCallback, useEffect, useRef } from 'react';
import { JP8Engine, P } from '../audio/jp8-engine';
import { setupMIDI } from '../synth/midi';
import { Slider } from './Slider';
import { ButtonGroup } from './ButtonGroup';
import { Keyboard } from './Keyboard';

const WAVE_OPTS = [
  { label: '⩘', value: 0 },  // Saw
  { label: '⌇', value: 1 },  // Pulse
  { label: '⊓', value: 2 },  // Square
];

const LFO_OPTS = [
  { label: '∿', value: 0 },
  { label: '△', value: 1 },
  { label: '⩘', value: 2 },
  { label: '⊓', value: 3 },
  { label: 'S&H', value: 4 },
];

const CHORUS_OPTS = [
  { label: 'OFF', value: 0 },
  { label: 'I', value: 1 },
  { label: 'II', value: 2 },
  { label: 'I+II', value: 3 },
];

const ASSIGN_OPTS = [
  { label: 'POLY', value: 0 },
  { label: 'UNI', value: 2 },
  { label: 'SOLO', value: 3 },
];

export function JP8Panel() {
  const engineRef = useRef<JP8Engine | null>(null);
  const [status, setStatus] = useState<string>('idle');

  // All 32 params as state (spec §5.1 defaults)
  const [params, setParams] = useState<number[]>([
    0, 0, 0.5, 0.8,       // VCO1: wave, range, pw, level
    0, 0, 0.5, 0.8,       // VCO2: wave, range, pw, level
    0, 0, 0,               // detune, cross_mod, noise
    8000, 0, 0.5, 0.5,    // filter: cutoff, reso, env_depth, key_track
    0.01, 0.3, 0.6, 0.5,  // env1: A, D, S, R
    0.01, 0.3, 0.7, 0.5,  // env2: A, D, S, R
    5, 0, 0, 0, 0,        // LFO: rate, wave, pitch, filter, pwm
    3, 0.7, 0, 0,         // chorus, volume, assign, portamento
  ]);

  const engine = () => engineRef.current;

  const setP = useCallback((index: number, value: number) => {
    setParams(prev => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
    engine()?.setParam(index, value);
  }, []);

  const handleStart = useCallback(async () => {
    if (engineRef.current) return;
    const eng = new JP8Engine();
    engineRef.current = eng;
    eng.setStatusCallback(setStatus);
    await eng.start();
    if (eng.getStatus() === 'ready') {
      setupMIDI(eng);
    }
  }, []);

  useEffect(() => {
    return () => { engineRef.current?.stop(); };
  }, []);

  const noteOn = useCallback((note: number) => engine()?.noteOn(note), []);
  const noteOff = useCallback((note: number) => engine()?.noteOff(note), []);

  const isReady = status === 'ready';

  return (
    <div style={styles.outer}>
      <div style={styles.woodLeft} />
      <div style={styles.panel}>
        {/* Header */}
        <div style={styles.header}>
          <div style={styles.brand}>
            <span style={styles.brandName}>JUPITER-8</span>
            <span style={styles.brandSub}>Virtual Analog Synthesizer</span>
          </div>
          <div style={{ flex: 1 }} />
          {!isReady ? (
            <button style={styles.startButton} onClick={handleStart}>
              {status === 'loading' ? 'Loading...' : status === 'error' ? 'Error' : 'Start Audio'}
            </button>
          ) : (
            <div style={styles.statusBadge}>ACTIVE</div>
          )}
        </div>

        {/* Control Sections */}
        <div style={styles.sections}>
          {/* VCO-1 */}
          <Section title="VCO-1">
            <ButtonGroup label="Wave" options={WAVE_OPTS} selected={params[P.VCO1_WAVE]} onChange={v => setP(P.VCO1_WAVE, v)} />
            <Slider label="Range" value={params[P.VCO1_RANGE]} min={-2} max={2} step={1} onChange={v => setP(P.VCO1_RANGE, v)} />
            <Slider label="PW" value={params[P.VCO1_PW]} min={0.05} max={0.95} step={0.01} onChange={v => setP(P.VCO1_PW, v)} />
            <Slider label="Level" value={params[P.VCO1_LEVEL]} min={0} max={1} step={0.01} onChange={v => setP(P.VCO1_LEVEL, v)} />
          </Section>
          <Divider />

          {/* VCO-2 */}
          <Section title="VCO-2">
            <ButtonGroup label="Wave" options={WAVE_OPTS} selected={params[P.VCO2_WAVE]} onChange={v => setP(P.VCO2_WAVE, v)} />
            <Slider label="Range" value={params[P.VCO2_RANGE]} min={-2} max={2} step={1} onChange={v => setP(P.VCO2_RANGE, v)} />
            <Slider label="PW" value={params[P.VCO2_PW]} min={0.05} max={0.95} step={0.01} onChange={v => setP(P.VCO2_PW, v)} />
            <Slider label="Level" value={params[P.VCO2_LEVEL]} min={0} max={1} step={0.01} onChange={v => setP(P.VCO2_LEVEL, v)} />
            <Slider label="Detune" value={params[P.VCO2_DETUNE]} min={-1} max={1} step={0.01} onChange={v => setP(P.VCO2_DETUNE, v)} />
          </Section>
          <Divider />

          {/* MIXER */}
          <Section title="MIX">
            <Slider label="X-Mod" value={params[P.CROSS_MOD]} min={0} max={1} step={0.01} onChange={v => setP(P.CROSS_MOD, v)} />
            <Slider label="Noise" value={params[P.NOISE]} min={0} max={1} step={0.01} onChange={v => setP(P.NOISE, v)} />
          </Section>
          <Divider />

          {/* VCF */}
          <Section title="VCF">
            <Slider label="Cutoff" value={params[P.FILTER_CUTOFF]} min={20} max={20000} step={10} onChange={v => setP(P.FILTER_CUTOFF, v)} unit="Hz" displayValue={`${Math.round(params[P.FILTER_CUTOFF])}`} />
            <Slider label="Reso" value={params[P.FILTER_RESO]} min={0} max={1} step={0.01} onChange={v => setP(P.FILTER_RESO, v)} />
            <Slider label="Env" value={params[P.FILTER_ENV]} min={-1} max={1} step={0.01} onChange={v => setP(P.FILTER_ENV, v)} />
            <Slider label="Key" value={params[P.FILTER_KEY]} min={0} max={1} step={0.01} onChange={v => setP(P.FILTER_KEY, v)} />
          </Section>
          <Divider />

          {/* ENV-1 (Filter) */}
          <Section title="ENV-1">
            <Slider label="A" value={params[P.ENV1_A]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_A, v)} />
            <Slider label="D" value={params[P.ENV1_D]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_D, v)} />
            <Slider label="S" value={params[P.ENV1_S]} min={0} max={1} step={0.01} onChange={v => setP(P.ENV1_S, v)} />
            <Slider label="R" value={params[P.ENV1_R]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_R, v)} />
          </Section>
          <Divider />

          {/* ENV-2 (Amp) */}
          <Section title="ENV-2">
            <Slider label="A" value={params[P.ENV2_A]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_A, v)} />
            <Slider label="D" value={params[P.ENV2_D]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_D, v)} />
            <Slider label="S" value={params[P.ENV2_S]} min={0} max={1} step={0.01} onChange={v => setP(P.ENV2_S, v)} />
            <Slider label="R" value={params[P.ENV2_R]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_R, v)} />
          </Section>
          <Divider />

          {/* LFO */}
          <Section title="LFO">
            <ButtonGroup label="Wave" options={LFO_OPTS} selected={params[P.LFO_WAVE]} onChange={v => setP(P.LFO_WAVE, v)} />
            <Slider label="Rate" value={params[P.LFO_RATE]} min={0.1} max={30} step={0.1} onChange={v => setP(P.LFO_RATE, v)} unit="Hz" />
            <Slider label="Pitch" value={params[P.LFO_PITCH]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_PITCH, v)} />
            <Slider label="Filter" value={params[P.LFO_FILTER]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_FILTER, v)} />
            <Slider label="PWM" value={params[P.LFO_PWM]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_PWM, v)} />
          </Section>
          <Divider />

          {/* CHORUS + MASTER */}
          <Section title="OUTPUT">
            <ButtonGroup label="Chorus" options={CHORUS_OPTS} selected={params[P.CHORUS]} onChange={v => setP(P.CHORUS, v)} />
            <ButtonGroup label="Assign" options={ASSIGN_OPTS} selected={params[P.ASSIGN]} onChange={v => setP(P.ASSIGN, v)} />
            <Slider label="Volume" value={params[P.VOLUME]} min={0} max={1} step={0.01} onChange={v => setP(P.VOLUME, v)} />
          </Section>
        </div>

        {/* Keyboard */}
        <div style={styles.keyboardSection}>
          <Keyboard onNoteOn={noteOn} onNoteOff={noteOff} />
        </div>
      </div>
      <div style={styles.woodRight} />
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={styles.section}>
      <div style={styles.sectionTitle}>{title}</div>
      <div style={styles.sectionContent}>{children}</div>
    </div>
  );
}

function Divider() { return <div style={styles.divider} />; }

const WOOD = 'linear-gradient(180deg, #8B6914 0%, #654B0F 30%, #7A5C17 50%, #654B0F 70%, #8B6914 100%)';

const styles: Record<string, React.CSSProperties> = {
  outer: { display: 'flex', justifyContent: 'center', alignItems: 'stretch', minHeight: '100vh', backgroundColor: '#0a0a0a', padding: '20px 0' },
  woodLeft: { width: 24, background: WOOD, borderRadius: '8px 0 0 8px', boxShadow: 'inset -2px 0 4px rgba(0,0,0,0.3)' },
  woodRight: { width: 24, background: WOOD, borderRadius: '0 8px 8px 0', boxShadow: 'inset 2px 0 4px rgba(0,0,0,0.3)' },
  panel: { backgroundColor: '#2d2d2d', backgroundImage: 'linear-gradient(180deg, #353535 0%, #2a2a2a 10%, #2d2d2d 100%)', padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 16, minWidth: 900, maxWidth: 1200, boxShadow: '0 4px 20px rgba(0,0,0,0.5)' },
  header: { display: 'flex', alignItems: 'center', gap: 16, borderBottom: '2px solid #444', paddingBottom: 12 },
  brand: { display: 'flex', flexDirection: 'column' },
  brandName: { fontFamily: 'Orbitron, monospace', fontSize: 22, fontWeight: 700, color: '#e8a045', letterSpacing: '0.1em' },
  brandSub: { fontSize: 9, color: '#888', letterSpacing: '0.15em', textTransform: 'uppercase' },
  startButton: { padding: '8px 20px', fontSize: 13, fontWeight: 600, fontFamily: 'Inter, sans-serif', color: '#1a1a1a', backgroundColor: '#e8a045', border: 'none', borderRadius: 6, cursor: 'pointer', whiteSpace: 'nowrap' },
  statusBadge: { fontSize: 11, fontWeight: 600, color: '#4ade80', whiteSpace: 'nowrap' },
  sections: { display: 'flex', alignItems: 'flex-start', gap: 0, padding: '8px 0', overflowX: 'auto' },
  section: { display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8, padding: '0 6px', flexShrink: 0 },
  sectionTitle: { fontSize: 10, fontWeight: 700, color: '#e8a045', textTransform: 'uppercase', letterSpacing: '0.12em', borderBottom: '1px solid #555', paddingBottom: 4, width: '100%', textAlign: 'center' },
  sectionContent: { display: 'flex', alignItems: 'flex-start', gap: 4 },
  divider: { width: 1, alignSelf: 'stretch', backgroundColor: '#444', margin: '0 2px', flexShrink: 0 },
  keyboardSection: { borderTop: '2px solid #444', paddingTop: 12, display: 'flex', justifyContent: 'center' },
};
