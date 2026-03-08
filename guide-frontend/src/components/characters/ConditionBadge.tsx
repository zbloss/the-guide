import type { Condition } from '../../api/types';

const CONDITION_ICONS: Partial<Record<Condition, string>> = {
  Blinded: '👁️',
  Charmed: '💜',
  Deafened: '🔇',
  Exhausted: '😴',
  Frightened: '😱',
  Grappled: '🤼',
  Incapacitated: '💫',
  Invisible: '👻',
  Paralyzed: '⚡',
  Petrified: '🪨',
  Poisoned: '☠️',
  Prone: '⬇️',
  Restrained: '⛓️',
  Stunned: '⭐',
  Unconscious: '💤',
};

interface ConditionBadgeProps {
  condition: Condition;
  onRemove?: (c: Condition) => void;
}

export function ConditionBadge({ condition, onRemove }: ConditionBadgeProps) {
  return (
    <span className="condition-badge">
      <span>{CONDITION_ICONS[condition] ?? '⚠️'}</span>
      <span>{condition}</span>
      {onRemove && (
        <button className="condition-badge-remove" onClick={() => onRemove(condition)} aria-label={`Remove ${condition}`}>
          ×
        </button>
      )}
    </span>
  );
}
