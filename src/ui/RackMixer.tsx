/**
 * Compact mixer strip — always visible below the synth panel.
 * One column per rack instance: name, volume fader, pan, M/S, MIDI ch.
 */

import type { RackInstance } from '../audio/jp8-rack';
import { HSlider } from './HSlider';

interface RackMixerProps {
  instances: RackInstance[];
  activeId: number;
  onSelectInstance: (id: number) => void;
  onVolumeChange: (id: number, vol: number) => void;
  onPanChange: (id: number, pan: number) => void;
  onMuteToggle: (id: number) => void;
  onSoloToggle: (id: number) => void;
  onMidiChannelChange: (id: number, ch: number) => void;
}

export function RackMixer({
  instances,
  activeId,
  onSelectInstance,
  onVolumeChange,
  onPanChange,
  onMuteToggle,
  onSoloToggle,
  onMidiChannelChange,
}: RackMixerProps) {
  return (
    <div style={styles.strip}>
      {instances.map((inst) => (
        <div
          key={inst.id}
          style={{
            ...styles.channel,
            ...(inst.id === activeId ? styles.channelActive : {}),
          }}
          onClick={() => onSelectInstance(inst.id)}
        >
          <div style={styles.name}>{inst.name}</div>

          <HSlider
            label="Vol"
            value={inst.channel.volume}
            min={0} max={1} step={0.01}
            onChange={(v) => onVolumeChange(inst.id, v)}
            width={50}
          />

          <HSlider
            label="Pan"
            value={inst.channel.pan}
            min={-1} max={1} step={0.01}
            onChange={(v) => onPanChange(inst.id, v)}
            width={50}
          />

          <div style={styles.buttons}>
            <button
              style={{ ...styles.btn, ...(inst.channel.mute ? styles.btnMute : {}) }}
              onClick={(e) => { e.stopPropagation(); onMuteToggle(inst.id); }}
            >M</button>
            <button
              style={{ ...styles.btn, ...(inst.channel.solo ? styles.btnSolo : {}) }}
              onClick={(e) => { e.stopPropagation(); onSoloToggle(inst.id); }}
            >S</button>
          </div>

          <select
            style={styles.midiSelect}
            value={inst.channel.midiChannel}
            onChange={(e) => { e.stopPropagation(); onMidiChannelChange(inst.id, Number(e.target.value)); }}
            onClick={(e) => e.stopPropagation()}
          >
            <option value={0}>OMNI</option>
            {Array.from({ length: 16 }, (_, i) => (
              <option key={i + 1} value={i + 1}>CH {i + 1}</option>
            ))}
          </select>
        </div>
      ))}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  strip: {
    display: 'flex',
    gap: 4,
    padding: '8px 0',
    borderTop: '2px solid #444',
    overflowX: 'auto',
  },
  channel: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: 4,
    padding: '6px 8px',
    backgroundColor: '#1e1e1e',
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: '#333',
    borderRadius: 4,
    cursor: 'pointer',
    minWidth: 90,
    transition: 'border-color 0.1s',
  },
  channelActive: {
    borderColor: '#e8a045',
  },
  name: {
    fontSize: 9,
    fontWeight: 700,
    color: '#e8a045',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    textAlign: 'center',
    whiteSpace: 'nowrap',
  },
  buttons: {
    display: 'flex',
    gap: 3,
  },
  btn: {
    width: 22,
    height: 18,
    fontSize: 8,
    fontWeight: 700,
    fontFamily: 'Inter, sans-serif',
    color: '#888',
    backgroundColor: '#2a2a2a',
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: '#444',
    borderRadius: 2,
    cursor: 'pointer',
    padding: 0,
  },
  btnMute: {
    backgroundColor: '#dc2626',
    color: '#fff',
    borderColor: '#dc2626',
  },
  btnSolo: {
    backgroundColor: '#eab308',
    color: '#1a1a1a',
    borderColor: '#eab308',
  },
  midiSelect: {
    width: '100%',
    fontSize: 8,
    fontFamily: 'Inter, sans-serif',
    backgroundColor: '#2a2a2a',
    color: '#aaa',
    border: '1px solid #444',
    borderRadius: 2,
    padding: '1px 2px',
    cursor: 'pointer',
  },
};
