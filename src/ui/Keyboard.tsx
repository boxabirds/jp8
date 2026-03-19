import { useCallback, useRef } from 'react';

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

/**
 * On-screen piano keyboard with pointer and keyboard input.
 */
export function Keyboard({
  onNoteOn,
  onNoteOff,
  startNote = START_NOTE_DEFAULT,
  numOctaves = NUM_OCTAVES_DEFAULT,
}: KeyboardProps) {
  const activeNotes = useRef(new Set<number>());
  const endNote = startNote + numOctaves * 12;

  const whiteKeys: number[] = [];
  const blackKeys: number[] = [];
  for (let n = startNote; n < endNote; n++) {
    if (isBlackKey(n)) blackKeys.push(n);
    else whiteKeys.push(n);
  }

  const WHITE_KEY_WIDTH = 28;
  const totalWidth = whiteKeys.length * WHITE_KEY_WIDTH;

  const handleDown = useCallback(
    (note: number) => {
      if (!activeNotes.current.has(note)) {
        activeNotes.current.add(note);
        onNoteOn(note);
      }
    },
    [onNoteOn],
  );

  const handleUp = useCallback(
    (note: number) => {
      if (activeNotes.current.has(note)) {
        activeNotes.current.delete(note);
        onNoteOff(note);
      }
    },
    [onNoteOff],
  );

  // Computer keyboard mapping (ZSXDCVGBHNJM... → notes)
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

  // Keyboard listeners
  const keyboardActive = useRef(new Set<string>());

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.repeat) return;
      const key = e.key.toLowerCase();
      const note = keyMap.current.get(key);
      if (note !== undefined && !keyboardActive.current.has(key)) {
        keyboardActive.current.add(key);
        handleDown(note);
      }
    },
    [handleDown],
  );

  const onKeyUp = useCallback(
    (e: React.KeyboardEvent) => {
      const key = e.key.toLowerCase();
      const note = keyMap.current.get(key);
      if (note !== undefined && keyboardActive.current.has(key)) {
        keyboardActive.current.delete(key);
        handleUp(note);
      }
    },
    [handleUp],
  );

  // White key x position
  const whiteKeyX = (idx: number) => idx * WHITE_KEY_WIDTH;

  // Black key position relative to white keys
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
      <div style={{ ...styles.keyboard, width: totalWidth }}>
        {/* White keys */}
        {whiteKeys.map((note, i) => (
          <div
            key={note}
            style={{
              ...styles.whiteKey,
              left: whiteKeyX(i),
            }}
            onPointerDown={() => handleDown(note)}
            onPointerUp={() => handleUp(note)}
            onPointerLeave={() => handleUp(note)}
          />
        ))}
        {/* Black keys */}
        {blackKeys.map((note) => (
          <div
            key={note}
            style={{
              ...styles.blackKey,
              left: blackKeyX(note),
            }}
            onPointerDown={() => handleDown(note)}
            onPointerUp={() => handleUp(note)}
            onPointerLeave={() => handleUp(note)}
          />
        ))}
      </div>
      <div style={styles.hint}>
        Click keys or use computer keyboard (Z-M lower, Q-P upper octave)
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
    height: 110,
    userSelect: 'none',
    touchAction: 'none',
  },
  whiteKey: {
    position: 'absolute',
    top: 0,
    width: 26,
    height: 110,
    backgroundColor: '#f0ece4',
    border: '1px solid #888',
    borderRadius: '0 0 4px 4px',
    cursor: 'pointer',
    transition: 'background-color 0.05s',
  },
  blackKey: {
    position: 'absolute',
    top: 0,
    width: 18,
    height: 68,
    backgroundColor: '#1a1a1a',
    border: '1px solid #333',
    borderRadius: '0 0 3px 3px',
    cursor: 'pointer',
    zIndex: 1,
    transition: 'background-color 0.05s',
  },
  hint: {
    fontSize: 10,
    color: '#666',
    marginTop: 6,
    textAlign: 'center',
  },
};
