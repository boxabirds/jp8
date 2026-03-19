import { useCallback, useRef } from 'react';

interface HSliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  width?: number;
}

export function HSlider({ label, value, min, max, step = 0.01, onChange, width = 100 }: HSliderProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const handlePointer = useCallback(
    (clientX: number) => {
      const track = trackRef.current;
      if (!track) return;
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      const raw = min + ratio * (max - min);
      const stepped = Math.round(raw / step) * step;
      onChange(Math.max(min, Math.min(max, stepped)));
    },
    [min, max, step, onChange],
  );

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      handlePointer(e.clientX);
    },
    [handlePointer],
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (dragging.current) handlePointer(e.clientX);
    },
    [handlePointer],
  );

  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const ratio = (value - min) / (max - min);

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6, userSelect: 'none' }}>
      <span style={{ fontSize: 9, fontWeight: 600, color: '#e8a045', textTransform: 'uppercase', letterSpacing: '0.05em', minWidth: 40 }}>
        {label}
      </span>
      <div
        ref={trackRef}
        style={{
          position: 'relative',
          width,
          height: 6,
          backgroundColor: '#1a1a1a',
          borderRadius: 3,
          border: '1px solid #444',
          cursor: 'pointer',
          touchAction: 'none',
        }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
      >
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          bottom: 0,
          width: `${ratio * 100}%`,
          backgroundColor: '#e8a045',
          borderRadius: 3,
          opacity: 0.6,
        }} />
        <div style={{
          position: 'absolute',
          top: '50%',
          left: `${ratio * 100}%`,
          transform: 'translate(-50%, -50%)',
          width: 10,
          height: 16,
          backgroundColor: '#d4d4d4',
          borderRadius: 2,
          border: '1px solid #888',
          boxShadow: '0 1px 3px rgba(0,0,0,0.5)',
        }} />
      </div>
    </div>
  );
}
