import type { Condition } from '../../api/types';

const CONDITION_ICONS: Partial<Record<Condition, string>> = {
  blinded: '👁️',
  charmed: '💜',
  deafened: '🔇',
  frightened: '😱',
  grappled: '🤼',
  incapacitated: '💫',
  invisible: '👻',
  paralyzed: '⚡',
  petrified: '🪨',
  poisoned: '☠️',
  prone: '⬇️',
  restrained: '⛓️',
  stunned: '⭐',
  unconscious: '💤',
};

interface ConditionBadgeProps {
  condition: Condition;
  onRemove?: (c: Condition) => void;
}

export function ConditionBadge({ condition, onRemove }: ConditionBadgeProps) {
  const label = condition.charAt(0).toUpperCase() + condition.slice(1);
  return (
    <span className="condition-badge">
      <span>{CONDITION_ICONS[condition] ?? '⚠️'}</span>
      <span>{label}</span>
      {onRemove && (
        <button className="condition-badge-remove" onClick={() => onRemove(condition)} aria-label={`Remove ${label}`}>
          ×
        </button>
      )}
    </span>
  );
}
