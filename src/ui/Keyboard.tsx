import { useCallback, useRef, useEffect } from 'react';

interface KeyboardProps {
  onNoteOn: (note: number) => void;
  onNoteOff: (note: number) => void;
  startNote?: number;
  numOctaves?: number;
}

const START_NOTE_DEFAULT = 48; // C3
const NUM_OCTAVES_DEFAULT = 3;

const isBlackKey = (note: number) => {
  const n = note % 12;
  return n === 1 || n === 3 || n === 6 || n === 8 || n === 10;
};

const blackKeyOffset: Record<number, number> = {
  1: 0.6, 3: 1.6, 6: 3.6, 8: 4.6, 10: 5.6,
};

export function Keyboard({
  onNoteOn,
  onNoteOff,
  startNote = START_NOTE_DEFAULT,
  numOctaves = NUM_OCTAVES_DEFAULT,
}: KeyboardProps) {
  const pointerDown = useRef(false);
  const currentNote = useRef<number | null>(null);
  const endNote = startNote + numOctaves * 12;

  const whiteKeys: number[] = [];
  const blackKeys: number[] = [];
  for (let n = startNote; n < endNote; n++) {
    if (isBlackKey(n)) blackKeys.push(n);
    else whiteKeys.push(n);
  }

  const WHITE_KEY_WIDTH = 28;
  const totalWidth = whiteKeys.length * WHITE_KEY_WIDTH;

  // Get MIDI note from a data-note attribute on a key element
  const noteFromPoint = useCallback((x: number, y: number): number | null => {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!el) return null;
    const noteStr = el.getAttribute('data-note');
    return noteStr !== null ? parseInt(noteStr, 10) : null;
  }, []);

  const triggerNote = useCallback((note: number | null) => {
    const prev = currentNote.current;
    if (note === prev) return;
    if (prev !== null) {
      onNoteOff(prev);
    }
    currentNote.current = note;
    if (note !== null) {
      onNoteOn(note);
    }
  }, [onNoteOn, onNoteOff]);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    pointerDown.current = true;
    // Don't setPointerCapture — we need elementFromPoint to find keys during drag
    e.preventDefault();
    const note = noteFromPoint(e.clientX, e.clientY);
    triggerNote(note);
  }, [noteFromPoint, triggerNote]);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    if (!pointerDown.current) return;
    const note = noteFromPoint(e.clientX, e.clientY);
    triggerNote(note);
  }, [noteFromPoint, triggerNote]);

  const onPointerUp = useCallback(() => {
    pointerDown.current = false;
    triggerNote(null);
  }, [triggerNote]);

  // Global pointerup to catch releases outside the keyboard area
  useEffect(() => {
    const handler = () => {
      if (pointerDown.current) {
        pointerDown.current = false;
        triggerNote(null);
      }
    };
    window.addEventListener('pointerup', handler);
    return () => window.removeEventListener('pointerup', handler);
  }, [triggerNote]);

  // Computer keyboard mapping
  const keyMap = useRef(new Map<string, number>());
  if (keyMap.current.size === 0) {
    const rows = [
      'z', 's', 'x', 'd', 'c', 'v', 'g', 'b', 'h', 'n', 'j', 'm',
      ',', 'l', '.', ';', '/',
      'q', '2', 'w', '3', 'e', 'r', '5', 't', '6', 'y', '7', 'u',
      'i', '9', 'o', '0', 'p',
    ];
    rows.forEach((key, i) => {
      keyMap.current.set(key, startNote + i);
    });
  }

  const keyboardActive = useRef(new Set<string>());

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.repeat) return;
      const key = e.key.toLowerCase();
      const note = keyMap.current.get(key);
      if (note !== undefined && !keyboardActive.current.has(key)) {
        keyboardActive.current.add(key);
        onNoteOn(note);
      }
    },
    [onNoteOn],
  );

  const onKeyUp = useCallback(
    (e: React.KeyboardEvent) => {
      const key = e.key.toLowerCase();
      const note = keyMap.current.get(key);
      if (note !== undefined && keyboardActive.current.has(key)) {
        keyboardActive.current.delete(key);
        onNoteOff(note);
      }
    },
    [onNoteOff],
  );

  const whiteKeyX = (idx: number) => idx * WHITE_KEY_WIDTH;

  const blackKeyX = (note: number) => {
    const octave = Math.floor((note - startNote) / 12);
    const semitone = note % 12;
    const offset = blackKeyOffset[semitone] ?? 0;
    return (octave * 7 + offset) * WHITE_KEY_WIDTH;
  };

  return (
    <div
      style={styles.container}
      tabIndex={0}
      onKeyDown={onKeyDown}
      onKeyUp={onKeyUp}
    >
      <div
        style={{ ...styles.keyboard, width: totalWidth }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
      >
        {whiteKeys.map((note, i) => (
          <div
            key={note}
            data-note={note}
            style={{ ...styles.whiteKey, left: whiteKeyX(i) }}
          />
        ))}
        {blackKeys.map((note) => (
          <div
            key={note}
            data-note={note}
            style={{ ...styles.blackKey, left: blackKeyX(note) }}
          />
        ))}
      </div>
      <div style={styles.hint}>
        Click &amp; drag keys, or use computer keyboard (Z-M lower, Q-P upper)
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    outline: 'none',
  },
  keyboard: {
    position: 'relative',
    height: 100,
    userSelect: 'none',
    touchAction: 'none',
  },
  whiteKey: {
    position: 'absolute',
    top: 0,
    width: 26,
    height: 100,
    backgroundColor: '#f0ece4',
    border: '1px solid #888',
    borderRadius: '0 0 4px 4px',
    cursor: 'pointer',
  },
  blackKey: {
    position: 'absolute',
    top: 0,
    width: 18,
    height: 62,
    backgroundColor: '#1a1a1a',
    border: '1px solid #333',
    borderRadius: '0 0 3px 3px',
    cursor: 'pointer',
    zIndex: 1,
  },
  hint: {
    fontSize: 10,
    color: '#666',
    marginTop: 6,
    textAlign: 'center',
  },
};
