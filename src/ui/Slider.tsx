import { useCallback, useRef, useEffect } from 'react';

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  unit?: string;
  displayValue?: string;
}

/**
 * Vertical slider styled like a JP-8 fader.
 */
export function Slider({
  label,
  value,
  min,
  max,
  step = 0.01,
  onChange,
  unit = '',
  displayValue,
}: SliderProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const handlePointer = useCallback(
    (clientY: number) => {
      const track = trackRef.current;
      if (!track) return;
      const rect = track.getBoundingClientRect();
      const ratio = 1 - (clientY - rect.top) / rect.height;
      const clamped = Math.max(0, Math.min(1, ratio));
      const raw = min + clamped * (max - min);
      const stepped = Math.round(raw / step) * step;
      onChange(Math.max(min, Math.min(max, stepped)));
    },
    [min, max, step, onChange],
  );

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      handlePointer(e.clientY);
    },
    [handlePointer],
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (dragging.current) handlePointer(e.clientY);
    },
    [handlePointer],
  );

  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const ratio = (value - min) / (max - min);
  const display = displayValue ?? `${value.toFixed(step >= 1 ? 0 : step >= 0.1 ? 1 : 2)}${unit}`;

  return (
    <div style={styles.container}>
      <div style={styles.label}>{label}</div>
      <div
        ref={trackRef}
        style={styles.track}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
      >
        <div style={{ ...styles.fill, height: `${ratio * 100}%` }} />
        <div style={{ ...styles.thumb, bottom: `${ratio * 100}%` }} />
      </div>
      <div style={styles.value}>{display}</div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: 4,
    width: 44,
    userSelect: 'none',
  },
  label: {
    fontSize: 9,
    fontWeight: 600,
    color: '#e8a045',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    textAlign: 'center',
    lineHeight: 1.2,
    minHeight: 22,
  },
  track: {
    position: 'relative',
    width: 6,
    height: 90,
    backgroundColor: '#1a1a1a',
    borderRadius: 3,
    cursor: 'pointer',
    border: '1px solid #444',
    touchAction: 'none',
  },
  fill: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    backgroundColor: '#e8a045',
    borderRadius: 3,
    opacity: 0.6,
  },
  thumb: {
    position: 'absolute',
    left: '50%',
    transform: 'translate(-50%, 50%)',
    width: 18,
    height: 10,
    backgroundColor: '#d4d4d4',
    borderRadius: 2,
    border: '1px solid #888',
    boxShadow: '0 1px 3px rgba(0,0,0,0.5)',
  },
  value: {
    fontSize: 8,
    color: '#999',
    textAlign: 'center',
    minHeight: 12,
  },
};
