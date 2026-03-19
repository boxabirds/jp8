interface ButtonGroupProps {
  label: string;
  options: { label: string; value: number }[];
  selected: number;
  onChange: (value: number) => void;
}

/**
 * Horizontal button group for waveform / mode selection.
 */
export function ButtonGroup({ label, options, selected, onChange }: ButtonGroupProps) {
  return (
    <div style={styles.container}>
      <div style={styles.label}>{label}</div>
      <div style={styles.buttons}>
        {options.map((opt) => (
          <button
            key={opt.value}
            style={{
              ...styles.button,
              ...(selected === opt.value ? styles.active : {}),
            }}
            onClick={() => onChange(opt.value)}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: 4,
  },
  label: {
    fontSize: 9,
    fontWeight: 600,
    color: '#e8a045',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
  },
  buttons: {
    display: 'flex',
    gap: 2,
  },
  button: {
    padding: '3px 6px',
    fontSize: 8,
    fontWeight: 600,
    fontFamily: 'Inter, sans-serif',
    color: '#ccc',
    backgroundColor: '#2a2a2a',
    border: '1px solid #555',
    borderRadius: 3,
    cursor: 'pointer',
    transition: 'all 0.1s',
    minWidth: 28,
    textAlign: 'center' as const,
  },
  active: {
    backgroundColor: '#e8a045',
    color: '#1a1a1a',
    borderColor: '#e8a045',
  },
};
