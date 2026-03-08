import type { Perspective } from '../../api/types';

interface PerspectiveSelectorProps {
  value: Perspective;
  onChange: (p: Perspective) => void;
  disabled?: boolean;
}

export function PerspectiveSelector({ value, onChange, disabled }: PerspectiveSelectorProps) {
  return (
    <div className="perspective-selector">
      <label className="radio-label">
        <input
          type="radio"
          name="perspective"
          value="dm"
          checked={value === 'dm'}
          onChange={() => onChange('dm')}
          disabled={disabled}
        />
        <span>DM View</span>
      </label>
      <label className="radio-label">
        <input
          type="radio"
          name="perspective"
          value="player"
          checked={value === 'player'}
          onChange={() => onChange('player')}
          disabled={disabled}
        />
        <span>Player View</span>
      </label>
    </div>
  );
}
