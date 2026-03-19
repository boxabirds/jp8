/**
 * RackView — Multi-instance JP-8 rack.
 * Tabbed instance switching + mixer strip.
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { JP8Rack, type RackInstance } from '../audio/jp8-rack';
import { setupMIDI } from '../synth/midi';
import { JP8Panel } from './JP8Panel';
import { RackMixer } from './RackMixer';

const DEFAULT_INSTANCE_COUNT = 2;

export function RackView() {
  const rackRef = useRef<JP8Rack | null>(null);
  const [status, setStatus] = useState<string>('idle');
  const [instances, setInstances] = useState<RackInstance[]>([]);
  const [activeId, setActiveId] = useState(0);
  const [, forceUpdate] = useState(0);

  const refreshInstances = useCallback(() => {
    const rack = rackRef.current;
    if (!rack) return;
    setInstances(rack.getInstances());
    setActiveId(rack.getActiveId());
  }, []);

  const handleStart = useCallback(async () => {
    if (rackRef.current) return;

    const rack = new JP8Rack('single-context');
    rackRef.current = rack;
    rack.setStatusCallback(setStatus);

    // Pre-create default instances
    for (let i = 0; i < DEFAULT_INSTANCE_COUNT; i++) {
      await rack.addInstance();
    }

    await rack.start();
    refreshInstances();

    // Set up MIDI routing to rack
    setupMIDI(rack);
  }, [refreshInstances]);

  const handleAddInstance = useCallback(async () => {
    const rack = rackRef.current;
    if (!rack) return;
    try {
      await rack.addInstance();
      refreshInstances();
    } catch (err) {
      console.warn('Cannot add instance:', err);
    }
  }, [refreshInstances]);

  const handleRemoveInstance = useCallback(async (id: number) => {
    const rack = rackRef.current;
    if (!rack) return;
    if (rack.getInstances().length <= 1) return; // keep at least 1
    await rack.removeInstance(id);
    refreshInstances();
  }, [refreshInstances]);

  const handleSelectInstance = useCallback((id: number) => {
    const rack = rackRef.current;
    if (!rack) return;
    rack.setActiveId(id);
    setActiveId(id);
  }, []);

  const handleVolumeChange = useCallback((id: number, vol: number) => {
    rackRef.current?.setChannelVolume(id, vol);
    forceUpdate(n => n + 1);
  }, []);

  const handlePanChange = useCallback((id: number, pan: number) => {
    rackRef.current?.setChannelPan(id, pan);
    forceUpdate(n => n + 1);
  }, []);

  const handleMuteToggle = useCallback((id: number) => {
    const inst = rackRef.current?.getInstances().find(i => i.id === id);
    if (inst) {
      rackRef.current?.setChannelMute(id, !inst.channel.mute);
      forceUpdate(n => n + 1);
    }
  }, []);

  const handleSoloToggle = useCallback((id: number) => {
    const inst = rackRef.current?.getInstances().find(i => i.id === id);
    if (inst) {
      rackRef.current?.setChannelSolo(id, !inst.channel.solo);
      forceUpdate(n => n + 1);
    }
  }, []);

  const handleMidiChannelChange = useCallback((id: number, ch: number) => {
    rackRef.current?.setChannelMidiChannel(id, ch);
    forceUpdate(n => n + 1);
  }, []);

  useEffect(() => {
    return () => { rackRef.current?.stop(); };
  }, []);

  const isReady = status === 'ready';
  const activeInstance = instances.find(i => i.id === activeId);

  return (
    <div style={styles.outer}>
      <div style={styles.woodLeft} />
      <div style={styles.panel}>
        {/* Top bar: Start + Tabs */}
        <div style={styles.topBar}>
          {!isReady ? (
            <button style={styles.startButton} onClick={handleStart} data-testid="start-audio">
              {status === 'loading' ? 'Loading...' : status === 'error' ? 'Error' : 'Start Audio'}
            </button>
          ) : (
            <>
              <div style={styles.tabs} data-testid="instance-tabs">
                {instances.map((inst) => (
                  <button
                    key={inst.id}
                    style={{
                      ...styles.tab,
                      ...(inst.id === activeId ? styles.tabActive : {}),
                    }}
                    onClick={() => handleSelectInstance(inst.id)}
                    data-testid={`tab-${inst.id}`}
                  >
                    {inst.name}
                    {instances.length > 1 && (
                      <span
                        style={styles.tabClose}
                        onClick={(e) => { e.stopPropagation(); handleRemoveInstance(inst.id); }}
                      >×</span>
                    )}
                  </button>
                ))}
                {instances.length < (rackRef.current?.getMaxInstances() ?? 8) && (
                  <button style={styles.addTab} onClick={handleAddInstance} data-testid="add-instance">+</button>
                )}
              </div>
              <div style={styles.statusBadge}>ACTIVE</div>
            </>
          )}
        </div>

        {/* Active instance panel */}
        {isReady && activeInstance && (
          <JP8Panel engine={activeInstance.engine} />
        )}

        {/* Mixer strip */}
        {isReady && instances.length > 0 && (
          <div data-testid="mixer-strip">
            <RackMixer
              instances={rackRef.current?.getInstances() ?? []}
              activeId={activeId}
              onSelectInstance={handleSelectInstance}
              onVolumeChange={handleVolumeChange}
              onPanChange={handlePanChange}
              onMuteToggle={handleMuteToggle}
              onSoloToggle={handleSoloToggle}
              onMidiChannelChange={handleMidiChannelChange}
            />
          </div>
        )}
      </div>
      <div style={styles.woodRight} />
    </div>
  );
}

const WOOD = 'linear-gradient(180deg, #8B6914 0%, #654B0F 30%, #7A5C17 50%, #654B0F 70%, #8B6914 100%)';

const styles: Record<string, React.CSSProperties> = {
  outer: { display: 'flex', justifyContent: 'center', alignItems: 'stretch', minHeight: '100vh', backgroundColor: '#0a0a0a', padding: '20px 0' },
  woodLeft: { width: 24, background: WOOD, borderRadius: '8px 0 0 8px', boxShadow: 'inset -2px 0 4px rgba(0,0,0,0.3)' },
  woodRight: { width: 24, background: WOOD, borderRadius: '0 8px 8px 0', boxShadow: 'inset 2px 0 4px rgba(0,0,0,0.3)' },
  panel: { backgroundColor: '#2d2d2d', backgroundImage: 'linear-gradient(180deg, #353535 0%, #2a2a2a 10%, #2d2d2d 100%)', padding: '12px 20px', display: 'flex', flexDirection: 'column', gap: 8, minWidth: 900, maxWidth: 1200, boxShadow: '0 4px 20px rgba(0,0,0,0.5)' },
  topBar: { display: 'flex', alignItems: 'center', gap: 8, borderBottom: '2px solid #444', paddingBottom: 8 },
  startButton: { padding: '10px 28px', fontSize: 14, fontWeight: 600, fontFamily: 'Inter, sans-serif', color: '#1a1a1a', backgroundColor: '#e8a045', border: 'none', borderRadius: 6, cursor: 'pointer' },
  tabs: { display: 'flex', gap: 3, flex: 1 },
  tab: { padding: '5px 12px', fontSize: 11, fontWeight: 500, fontFamily: 'Inter, sans-serif', color: '#aaa', backgroundColor: '#1e1e1e', borderWidth: 1, borderStyle: 'solid', borderColor: '#444', borderRadius: '4px 4px 0 0', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6 },
  tabActive: { backgroundColor: '#e8a045', color: '#1a1a1a', fontWeight: 700, borderColor: '#e8a045' },
  tabClose: { fontSize: 13, cursor: 'pointer', opacity: 0.6, lineHeight: 1 },
  addTab: { padding: '5px 10px', fontSize: 14, fontWeight: 700, fontFamily: 'Inter, sans-serif', color: '#888', backgroundColor: '#1e1e1e', border: '1px solid #444', borderRadius: '4px 4px 0 0', cursor: 'pointer' },
  statusBadge: { fontSize: 11, fontWeight: 600, color: '#4ade80', whiteSpace: 'nowrap' },
};
