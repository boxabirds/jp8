/**
 * ModuleTray — compact horizontal controls for the expanded signal flow block.
 * Uses HSlider for space efficiency.
 */

import { P } from '../audio/jp8-engine';
import { HSlider } from './HSlider';
import { ButtonGroup } from './ButtonGroup';

const EXCITATION_OPTS = [
  { label: 'Anvil', value: 0 },
  { label: 'Hiss', value: 1 },
  { label: 'Pop', value: 2 },
  { label: 'Twang', value: 3 },
  { label: 'Hit', value: 4 },
  { label: 'Click', value: 5 },
];

const BODY_OPTS = [
  { label: 'Metal', value: 0 },
  { label: 'Tube', value: 1 },
  { label: 'Glass', value: 2 },
  { label: 'Bell', value: 3 },
  { label: 'Wine', value: 4 },
];

const SPECTRAL_TARGET_OPTS = [
  { label: 'Saw', value: 0 },
  { label: 'Bell', value: 1 },
  { label: 'Voice', value: 2 },
  { label: 'Organ', value: 3 },
];

interface ModuleTrayProps {
  block: string;
  params: number[];
  setP: (index: number, value: number) => void;
}

export function ModuleTray({ block, params, setP }: ModuleTrayProps) {
  switch (block) {
    case 'source': return <SourceTray params={params} setP={setP} />;
    case 'bubble': return <BubbleTray params={params} setP={setP} />;
    case 'modal': return <ModalTray params={params} setP={setP} />;
    case 'chaos': return <ChaosTray params={params} setP={setP} />;
    default: return null;
  }
}

type TrayProps = { params: number[]; setP: (i: number, v: number) => void };

function TrayRow({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={styles.row}>
      <span style={styles.title}>{title}</span>
      <div style={styles.controls}>{children}</div>
    </div>
  );
}

function SourceTray({ params, setP }: TrayProps) {
  const mode = params[P.SOURCE_MODE] ?? 0;

  if (mode === 1) {
    return (
      <TrayRow title="SPECTRAL">
        <HSlider label="Tilt" value={params[P.SPECTRAL_TILT]} min={-1} max={1} step={0.01} onChange={v => setP(P.SPECTRAL_TILT, v)} width={70} />
        <HSlider label="Parts" value={params[P.SPECTRAL_PARTIALS]} min={2} max={64} step={1} onChange={v => setP(P.SPECTRAL_PARTIALS, v)} width={60} />
        <HSlider label="Noise" value={params[P.SPECTRAL_NOISE]} min={0} max={1} step={0.01} onChange={v => setP(P.SPECTRAL_NOISE, v)} width={60} />
        <HSlider label="Morph" value={params[P.SPECTRAL_MORPH]} min={0} max={1} step={0.01} onChange={v => setP(P.SPECTRAL_MORPH, v)} width={60} />
        <ButtonGroup label="Target" options={SPECTRAL_TARGET_OPTS} selected={params[P.SPECTRAL_TARGET]} onChange={v => setP(P.SPECTRAL_TARGET, v)} />
      </TrayRow>
    );
  }

  if (mode === 2) {
    return (
      <TrayRow title="WAVEGUIDE">
        <ButtonGroup label="Excite" options={EXCITATION_OPTS} selected={params[P.WG_EXCITATION]} onChange={v => setP(P.WG_EXCITATION, v)} />
        <ButtonGroup label="Body" options={BODY_OPTS} selected={params[P.WG_BODY]} onChange={v => setP(P.WG_BODY, v)} />
        <HSlider label="Bright" value={params[P.WG_BRIGHTNESS]} min={0} max={1} step={0.01} onChange={v => setP(P.WG_BRIGHTNESS, v)} width={60} />
        <HSlider label="Mix" value={params[P.WG_BODY_MIX]} min={0} max={1} step={0.01} onChange={v => setP(P.WG_BODY_MIX, v)} width={60} />
      </TrayRow>
    );
  }

  return null;
}

function BubbleTray({ params, setP }: TrayProps) {
  return (
    <TrayRow title="BUBBLE">
      <HSlider label="Rate" value={params[P.BUBBLE_RATE]} min={0} max={60} step={1} onChange={v => setP(P.BUBBLE_RATE, v)} width={60} />
      <HSlider label="MinSz" value={params[P.BUBBLE_MIN_SIZE]} min={0.001} max={0.01} step={0.001} onChange={v => setP(P.BUBBLE_MIN_SIZE, v)} width={60} />
      <HSlider label="MaxSz" value={params[P.BUBBLE_MAX_SIZE]} min={0.005} max={0.03} step={0.001} onChange={v => setP(P.BUBBLE_MAX_SIZE, v)} width={60} />
      <HSlider label="Level" value={params[P.BUBBLE_LEVEL]} min={0} max={1} step={0.01} onChange={v => setP(P.BUBBLE_LEVEL, v)} width={60} />
    </TrayRow>
  );
}

function ModalTray({ params, setP }: TrayProps) {
  return (
    <TrayRow title="MODAL">
      <HSlider label="Mix" value={params[P.MODAL_MIX]} min={0} max={1} step={0.01} onChange={v => setP(P.MODAL_MIX, v)} width={60} />
      <HSlider label="Mater" value={params[P.MODAL_MATERIAL]} min={0} max={1} step={0.01} onChange={v => setP(P.MODAL_MATERIAL, v)} width={60} />
      <ButtonGroup label="Body" options={BODY_OPTS} selected={params[P.MODAL_BODY]} onChange={v => setP(P.MODAL_BODY, v)} />
      <HSlider label="Modes" value={params[P.MODAL_MODES]} min={4} max={32} step={1} onChange={v => setP(P.MODAL_MODES, v)} width={60} />
      <HSlider label="Inharm" value={params[P.MODAL_INHARMONICITY]} min={0} max={1} step={0.01} onChange={v => setP(P.MODAL_INHARMONICITY, v)} width={60} />
    </TrayRow>
  );
}

function ChaosTray({ params, setP }: TrayProps) {
  return (
    <TrayRow title="CHAOS">
      <HSlider label="Rate1" value={params[P.CHAOS_RATE1]} min={0.1} max={30} step={0.1} onChange={v => setP(P.CHAOS_RATE1, v)} width={55} />
      <HSlider label="Rate2" value={params[P.CHAOS_RATE2]} min={0.1} max={30} step={0.1} onChange={v => setP(P.CHAOS_RATE2, v)} width={55} />
      <HSlider label="Depth" value={params[P.CHAOS_DEPTH]} min={0} max={1} step={0.01} onChange={v => setP(P.CHAOS_DEPTH, v)} width={55} />
      <HSlider label="→Pitch" value={params[P.CHAOS_TO_PITCH]} min={0} max={1} step={0.01} onChange={v => setP(P.CHAOS_TO_PITCH, v)} width={55} />
      <HSlider label="→Filt" value={params[P.CHAOS_TO_FILTER]} min={0} max={1} step={0.01} onChange={v => setP(P.CHAOS_TO_FILTER, v)} width={55} />
      <HSlider label="→PWM" value={params[P.CHAOS_TO_PWM]} min={0} max={1} step={0.01} onChange={v => setP(P.CHAOS_TO_PWM, v)} width={55} />
    </TrayRow>
  );
}

const styles: Record<string, React.CSSProperties> = {
  row: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    flexWrap: 'wrap',
  },
  title: {
    fontSize: 8,
    fontWeight: 700,
    color: '#e8a045',
    textTransform: 'uppercase',
    letterSpacing: '0.1em',
    minWidth: 50,
  },
  controls: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    flexWrap: 'wrap',
  },
};
