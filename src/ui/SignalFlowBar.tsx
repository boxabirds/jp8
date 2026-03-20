/**
 * Signal Flow Bar — compact visual of the audio signal chain.
 * Single-line strip with expandable module trays.
 */

import { useState, useCallback } from 'react';
import { P } from '../audio/jp8-engine';
import { ModuleTray } from './ModuleTray';

type BlockId = 'source' | 'bubble' | 'modal' | 'chaos';

interface SignalFlowBarProps {
  params: number[];
  setP: (index: number, value: number) => void;
}

const SOURCE_OPTS = ['BLEP', 'SPEC', 'WG'];

export function SignalFlowBar({ params, setP }: SignalFlowBarProps) {
  const [expandedBlock, setExpandedBlock] = useState<BlockId | null>(null);

  const sourceMode = params[P.SOURCE_MODE] ?? 0;
  const modalActive = (params[P.MODAL_MIX] ?? 0) > 0;
  const chaosActive = (params[P.CHAOS_ENABLE] ?? 0) > 0.5;
  const bubbleActive = (params[P.BUBBLE_ENABLE] ?? 0) > 0.5;

  const toggleExpand = useCallback((id: BlockId) => {
    setExpandedBlock(prev => prev === id ? null : id);
  }, []);

  const handleSourceChange = useCallback((value: number) => {
    setP(P.SOURCE_MODE, value);
    // Only SPEC opens a tray (WG controls are inline in the sections)
    if (value === 1) {
      setExpandedBlock('source');
    } else if (expandedBlock === 'source') {
      setExpandedBlock(null);
    }
  }, [setP, expandedBlock]);

  const toggleModule = useCallback((enableParam: number, currentValue: number) => {
    setP(enableParam, currentValue > 0.5 ? 0 : 1);
  }, [setP]);

  return (
    <div data-testid="signal-flow-bar" style={styles.container}>
      {/* Strip */}
      <div style={styles.strip}>
        {/* SOURCE */}
        <div style={{ ...styles.block, ...(sourceMode !== 0 ? styles.active : {}), ...(expandedBlock === 'source' ? styles.expanded : {}) }} data-testid="sfb-source">
          <span style={styles.label}>SRC</span>
          {SOURCE_OPTS.map((name, i) => (
            <button key={i} style={{ ...styles.srcBtn, ...(sourceMode === i ? styles.srcBtnOn : {}) }} onClick={() => handleSourceChange(i)}>{name}</button>
          ))}
        </div>

        <span style={styles.arrow}>→</span>

        {/* BUBBLE */}
        <div style={{ ...styles.block, ...(bubbleActive ? styles.active : {}), ...(expandedBlock === 'bubble' ? styles.expanded : {}) }} data-testid="sfb-bubble">
          <span style={styles.label}>BUB</span>
          <button style={{ ...styles.tog, ...(bubbleActive ? styles.togOn : {}) }} onClick={() => toggleModule(P.BUBBLE_ENABLE, params[P.BUBBLE_ENABLE] ?? 0)} data-testid="sfb-bubble-toggle">{bubbleActive ? 'ON' : 'OFF'}</button>
          <button style={styles.exp} onClick={() => toggleExpand('bubble')} data-testid="sfb-bubble-expand">{expandedBlock === 'bubble' ? '▲' : '▼'}</button>
        </div>

        <span style={styles.arrow}>→</span>

        {/* VCF anchor */}
        <div style={{ ...styles.block, ...styles.anchor }} data-testid="sfb-vcf"><span style={styles.label}>VCF</span></div>

        <span style={styles.arrow}>→</span>

        {/* MODAL */}
        <div style={{ ...styles.block, ...(modalActive ? styles.active : {}), ...(expandedBlock === 'modal' ? styles.expanded : {}) }} data-testid="sfb-modal">
          <span style={styles.label}>MOD</span>
          <button style={{ ...styles.tog, ...(modalActive ? styles.togOn : {}) }} onClick={() => { const c = params[P.MODAL_MIX] ?? 0; setP(P.MODAL_MIX, c > 0 ? 0 : 0.5); }} data-testid="sfb-modal-toggle">{modalActive ? 'ON' : 'OFF'}</button>
          <button style={styles.exp} onClick={() => toggleExpand('modal')} data-testid="sfb-modal-expand">{expandedBlock === 'modal' ? '▲' : '▼'}</button>
        </div>

        <span style={styles.arrow}>→</span>

        {/* VCA→CHR anchor */}
        <div style={{ ...styles.block, ...styles.anchor }} data-testid="sfb-output"><span style={styles.label}>OUT</span></div>

        {/* Divider */}
        <div style={styles.divider} />

        {/* CHAOS */}
        <div style={{ ...styles.block, ...(chaosActive ? styles.active : {}), ...(expandedBlock === 'chaos' ? styles.expanded : {}) }} data-testid="sfb-chaos">
          <span style={styles.label}>CHAOS</span>
          <button style={{ ...styles.tog, ...(chaosActive ? styles.togOn : {}) }} onClick={() => toggleModule(P.CHAOS_ENABLE, params[P.CHAOS_ENABLE] ?? 0)} data-testid="sfb-chaos-toggle">{chaosActive ? 'ON' : 'OFF'}</button>
          <button style={styles.exp} onClick={() => toggleExpand('chaos')} data-testid="sfb-chaos-expand">{expandedBlock === 'chaos' ? '▲' : '▼'}</button>
        </div>
      </div>

      {/* Tray */}
      {expandedBlock && (
        <div style={styles.tray} data-testid="module-tray">
          <ModuleTray block={expandedBlock} params={params} setP={setP} />
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    borderTop: '1px solid #333',
  },
  strip: {
    display: 'flex',
    alignItems: 'center',
    gap: 3,
    padding: '3px 8px',
    flexWrap: 'wrap',
  },
  block: {
    display: 'flex',
    alignItems: 'center',
    gap: 3,
    padding: '2px 5px',
    borderRadius: 3,
    backgroundColor: '#1e1e1e',
    border: '1px solid #333',
    height: 22,
  },
  active: {
    borderColor: '#e8a045',
  },
  expanded: {
    borderWidth: 2,
    borderColor: '#e8a045',
  },
  anchor: {
    opacity: 0.4,
  },
  label: {
    fontSize: 7,
    fontWeight: 700,
    color: '#999',
    letterSpacing: '0.06em',
    textTransform: 'uppercase' as const,
  },
  srcBtn: {
    fontSize: 7,
    fontWeight: 600,
    padding: '1px 3px',
    border: '1px solid #444',
    borderRadius: 2,
    backgroundColor: '#2a2a2a',
    color: '#777',
    cursor: 'pointer',
    lineHeight: 1,
  },
  srcBtnOn: {
    backgroundColor: '#e8a045',
    color: '#1a1a1a',
    borderColor: '#e8a045',
  },
  tog: {
    fontSize: 7,
    fontWeight: 600,
    padding: '1px 3px',
    border: '1px solid #444',
    borderRadius: 2,
    backgroundColor: '#2a2a2a',
    color: '#666',
    cursor: 'pointer',
    lineHeight: 1,
  },
  togOn: {
    backgroundColor: '#e8a045',
    color: '#1a1a1a',
    borderColor: '#e8a045',
  },
  exp: {
    fontSize: 6,
    color: '#666',
    background: 'none',
    border: 'none',
    cursor: 'pointer',
    padding: 0,
    lineHeight: 1,
  },
  arrow: {
    color: '#555',
    fontSize: 10,
    lineHeight: 1,
    userSelect: 'none' as const,
  },
  divider: {
    width: 1,
    height: 16,
    backgroundColor: '#444',
    margin: '0 3px',
  },
  tray: {
    padding: '4px 8px',
    borderTop: '1px solid #333',
    borderBottom: '1px solid #333',
  },
};
