/**
 * JP-8 synth panel — receives engine as prop (owned by rack).
 * Initializes UI state from engine's SAB on mount.
 */

import { useState, useCallback, useEffect } from 'react';
import { JP8Engine, P } from '../audio/jp8-engine';
import { FACTORY_PATCHES } from '../synth/patches';
import { Slider } from './Slider';
import { ButtonGroup } from './ButtonGroup';
import { HSlider } from './HSlider';
import { Keyboard } from './Keyboard';

const WAVE_OPTS = [
  { label: '⩘', value: 1 },
  { label: '⌇', value: 2 },
  { label: '⩘+⌇', value: 3 },
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

const ARP_OPTS = [
  { label: 'OFF', value: 0 },
  { label: 'UP', value: 1 },
  { label: 'DN', value: 2 },
  { label: 'U/D', value: 3 },
];

const ARP_RANGE_OPTS = [
  { label: '1', value: 1 },
  { label: '2', value: 2 },
  { label: '3', value: 3 },
  { label: '4', value: 4 },
];

interface JP8PanelProps {
  engine: JP8Engine;
}

export function JP8Panel({ engine }: JP8PanelProps) {
  const [activePatch, setActivePatch] = useState(0);
  // Initialize from engine's current SAB state (preserves state across tab switches)
  const [params, setParams] = useState<number[]>(() => engine.getParams());

  // Re-read params when engine changes (tab switch)
  useEffect(() => {
    setParams(engine.getParams());
  }, [engine]);

  const setP = useCallback((index: number, value: number) => {
    setParams(prev => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
    engine.setParam(index, value);
  }, [engine]);

  const loadPatch = useCallback((index: number) => {
    const patch = FACTORY_PATCHES[index];
    if (!patch) return;
    setActivePatch(index);
    setParams([...patch.params]);
    for (let i = 0; i < patch.params.length; i++) {
      engine.setParam(i, patch.params[i]);
    }
  }, [engine]);

  const noteOn = useCallback((note: number) => engine.noteOn(note), [engine]);
  const noteOff = useCallback((note: number) => engine.noteOff(note), [engine]);

  return (
    <div>
      {/* Header row */}
      <div style={styles.header}>
        <div style={styles.brand}>
          <span style={styles.brandName}>JUPITER-8</span>
          <span style={styles.brandSub}>Virtual Analog Synthesizer</span>
        </div>
        <div style={{ flex: 1 }} />
        <div style={styles.headerControls}>
          <ButtonGroup label="Chorus" options={CHORUS_OPTS} selected={params[P.CHORUS]} onChange={v => setP(P.CHORUS, v)} />
          <ButtonGroup label="Assign" options={ASSIGN_OPTS} selected={params[P.ASSIGN]} onChange={v => setP(P.ASSIGN, v)} />
          <ButtonGroup label="Arp" options={ARP_OPTS} selected={params[P.ARP_MODE]} onChange={v => setP(P.ARP_MODE, v)} />
          <ButtonGroup label="Oct" options={ARP_RANGE_OPTS} selected={params[P.ARP_RANGE]} onChange={v => setP(P.ARP_RANGE, v)} />
          <HSlider label="Tempo" value={params[P.ARP_TEMPO]} min={30} max={300} step={1} onChange={v => setP(P.ARP_TEMPO, v)} width={60} />
          <HSlider label="Porta" value={params[P.PORTAMENTO]} min={0} max={5} step={0.01} onChange={v => setP(P.PORTAMENTO, v)} width={60} />
          <HSlider label="Volume" value={params[P.VOLUME]} min={0} max={1} step={0.01} onChange={v => setP(P.VOLUME, v)} width={90} />
        </div>
      </div>

      {/* Patch Bank */}
      <div style={styles.patchBank}>
        {FACTORY_PATCHES.map((patch, i) => (
          <button key={i} style={{ ...styles.patchButton, ...(activePatch === i ? styles.patchActive : {}) }} onClick={() => loadPatch(i)}>
            <span style={styles.patchNumber}>{String(i + 1).padStart(2, '0')}</span>
            <span>{patch.name}</span>
          </button>
        ))}
      </div>

      {/* Control Sections */}
      <div style={styles.sections}>
        <Section title="VCO-1">
          <ButtonGroup label="Wave" options={WAVE_OPTS} selected={params[P.VCO1_WAVE]} onChange={v => setP(P.VCO1_WAVE, v)} />
          <Slider label="Range" value={params[P.VCO1_RANGE]} min={-2} max={2} step={1} onChange={v => setP(P.VCO1_RANGE, v)} />
          <Slider label="PW" value={params[P.VCO1_PW]} min={0.05} max={0.95} step={0.01} onChange={v => setP(P.VCO1_PW, v)} />
          <Slider label="Level" value={params[P.VCO1_LEVEL]} min={0} max={1} step={0.01} onChange={v => setP(P.VCO1_LEVEL, v)} />
          <Slider label="Sub" value={params[P.SUB_OSC]} min={0} max={1} step={0.01} onChange={v => setP(P.SUB_OSC, v)} />
        </Section>
        <Divider />
        <Section title="VCO-2">
          <ButtonGroup label="Wave" options={WAVE_OPTS} selected={params[P.VCO2_WAVE]} onChange={v => setP(P.VCO2_WAVE, v)} />
          <Slider label="Range" value={params[P.VCO2_RANGE]} min={-2} max={2} step={1} onChange={v => setP(P.VCO2_RANGE, v)} />
          <Slider label="PW" value={params[P.VCO2_PW]} min={0.05} max={0.95} step={0.01} onChange={v => setP(P.VCO2_PW, v)} />
          <Slider label="Level" value={params[P.VCO2_LEVEL]} min={0} max={1} step={0.01} onChange={v => setP(P.VCO2_LEVEL, v)} />
          <Slider label="Detune" value={params[P.VCO2_DETUNE]} min={-1} max={1} step={0.01} onChange={v => setP(P.VCO2_DETUNE, v)} />
        </Section>
        <Divider />
        <Section title="MIX">
          <Slider label="X-Mod" value={params[P.CROSS_MOD]} min={0} max={1} step={0.01} onChange={v => setP(P.CROSS_MOD, v)} />
          <Slider label="Noise" value={params[P.NOISE]} min={0} max={1} step={0.01} onChange={v => setP(P.NOISE, v)} />
        </Section>
        <Divider />
        <Section title="HPF">
          <Slider label="Freq" value={params[P.HPF_CUTOFF]} min={20} max={6000} step={5} onChange={v => setP(P.HPF_CUTOFF, v)} displayValue={`${Math.round(params[P.HPF_CUTOFF])}`} />
        </Section>
        <Divider />
        <Section title="VCF">
          <Slider label="Cutoff" value={params[P.FILTER_CUTOFF]} min={20} max={20000} step={10} onChange={v => setP(P.FILTER_CUTOFF, v)} displayValue={`${Math.round(params[P.FILTER_CUTOFF])}`} />
          <Slider label="Reso" value={params[P.FILTER_RESO]} min={0} max={1} step={0.01} onChange={v => setP(P.FILTER_RESO, v)} />
          <Slider label="Env" value={params[P.FILTER_ENV]} min={-1} max={1} step={0.01} onChange={v => setP(P.FILTER_ENV, v)} />
          <Slider label="Key" value={params[P.FILTER_KEY]} min={0} max={1} step={0.01} onChange={v => setP(P.FILTER_KEY, v)} />
        </Section>
        <Divider />
        <Section title="ENV-1">
          <Slider label="A" value={params[P.ENV1_A]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_A, v)} />
          <Slider label="D" value={params[P.ENV1_D]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_D, v)} />
          <Slider label="S" value={params[P.ENV1_S]} min={0} max={1} step={0.01} onChange={v => setP(P.ENV1_S, v)} />
          <Slider label="R" value={params[P.ENV1_R]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV1_R, v)} />
          <ButtonGroup label="→VCA" options={[{label:'OFF',value:0},{label:'ON',value:1}]} selected={params[P.ENV1_VCA]} onChange={v => setP(P.ENV1_VCA, v)} />
        </Section>
        <Divider />
        <Section title="ENV-2">
          <Slider label="A" value={params[P.ENV2_A]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_A, v)} />
          <Slider label="D" value={params[P.ENV2_D]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_D, v)} />
          <Slider label="S" value={params[P.ENV2_S]} min={0} max={1} step={0.01} onChange={v => setP(P.ENV2_S, v)} />
          <Slider label="R" value={params[P.ENV2_R]} min={0.001} max={10} step={0.001} onChange={v => setP(P.ENV2_R, v)} />
        </Section>
        <Divider />
        <Section title="LFO">
          <ButtonGroup label="Wave" options={LFO_OPTS} selected={params[P.LFO_WAVE]} onChange={v => setP(P.LFO_WAVE, v)} />
          <Slider label="Rate" value={params[P.LFO_RATE]} min={0.1} max={30} step={0.1} onChange={v => setP(P.LFO_RATE, v)} unit="Hz" />
          <Slider label="Delay" value={params[P.LFO_DELAY]} min={0} max={5} step={0.01} onChange={v => setP(P.LFO_DELAY, v)} unit="s" />
          <Slider label="Pitch" value={params[P.LFO_PITCH]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_PITCH, v)} />
          <Slider label="Filter" value={params[P.LFO_FILTER]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_FILTER, v)} />
          <Slider label="PWM" value={params[P.LFO_PWM]} min={0} max={1} step={0.01} onChange={v => setP(P.LFO_PWM, v)} />
        </Section>
      </div>

      {/* Keyboard */}
      <div style={styles.keyboardSection}>
        <Keyboard onNoteOn={noteOn} onNoteOff={noteOff} />
      </div>
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

const styles: Record<string, React.CSSProperties> = {
  header: { display: 'flex', alignItems: 'center', gap: 16, borderBottom: '2px solid #444', paddingBottom: 8, flexWrap: 'wrap' },
  headerControls: { display: 'flex', alignItems: 'center', gap: 12 },
  brand: { display: 'flex', flexDirection: 'column' },
  brandName: { fontFamily: 'Orbitron, monospace', fontSize: 22, fontWeight: 700, color: '#e8a045', letterSpacing: '0.1em' },
  brandSub: { fontSize: 9, color: '#888', letterSpacing: '0.15em', textTransform: 'uppercase' },
  patchBank: { display: 'flex', flexWrap: 'wrap', gap: 3, padding: '4px 0', borderBottom: '1px solid #333' },
  patchButton: { padding: '3px 8px', fontSize: 9, fontWeight: 500, fontFamily: 'Inter, sans-serif', color: '#999', backgroundColor: '#1a1a1a', border: '1px solid #333', borderRadius: 3, cursor: 'pointer', display: 'flex', gap: 4, alignItems: 'center', transition: 'all 0.1s' },
  patchActive: { backgroundColor: '#e8a045', color: '#1a1a1a', borderColor: '#e8a045', fontWeight: 700 },
  patchNumber: { fontSize: 8, opacity: 0.6, fontFamily: 'monospace' },
  sections: { display: 'flex', alignItems: 'flex-start', gap: 0, padding: '8px 0', flexWrap: 'wrap', justifyContent: 'center' },
  section: { display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, padding: '0 5px' },
  sectionTitle: { fontSize: 10, fontWeight: 700, color: '#e8a045', textTransform: 'uppercase', letterSpacing: '0.12em', borderBottom: '1px solid #555', paddingBottom: 4, width: '100%', textAlign: 'center' },
  sectionContent: { display: 'flex', alignItems: 'flex-start', gap: 4 },
  divider: { width: 1, alignSelf: 'stretch', backgroundColor: '#444', margin: '0 2px', flexShrink: 0 },
  keyboardSection: { borderTop: '2px solid #444', paddingTop: 12, display: 'flex', justifyContent: 'center' },
};
