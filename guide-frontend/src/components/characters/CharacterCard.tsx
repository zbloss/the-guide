import { Link } from 'react-router-dom';
import { Badge } from '../common/Badge';
import { ConditionBadge } from './ConditionBadge';
import type { Character } from '../../api/types';

function HpBar({ current, max }: { current: number; max: number }) {
  const pct = max > 0 ? (current / max) * 100 : 0;
  const cls = pct > 50 ? 'hp-high' : pct > 25 ? 'hp-mid' : 'hp-low';
  return (
    <div className="hp-bar-container">
      <div className={`hp-bar-fill ${cls}`} style={{ width: `${Math.min(100, pct)}%` }} />
      <span className="hp-bar-label">{current}/{max}</span>
    </div>
  );
}

interface CharacterCardProps {
  character: Character;
  campaignId: string;
}

export function CharacterCard({ character, campaignId }: CharacterCardProps) {
  return (
    <div className={`card character-card ${!character.is_alive ? 'character-dead' : ''}`}>
      <div className="card-header">
        <Link to={`/campaigns/${campaignId}/characters/${character.id}`} className="card-title">
          {character.name}
        </Link>
        <div className="card-badges">
          <Badge label={character.character_type} variant="info" />
          {character.class && <Badge label={character.class} />}
          {!character.is_alive && <Badge label="Dead" variant="danger" />}
        </div>
      </div>
      <div className="character-stats">
        <span>Lvl {character.level}</span>
        {character.race && <span>{character.race}</span>}
        <span>AC {character.armor_class}</span>
      </div>
      <HpBar current={character.current_hp} max={character.max_hp} />
      {character.conditions.length > 0 && (
        <div className="condition-list">
          {character.conditions.map((c) => <ConditionBadge key={c} condition={c} />)}
        </div>
      )}
    </div>
  );
}
