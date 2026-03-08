import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getCharacter, updateCharacter } from '../api/characters';
import { ConditionBadge } from '../components/characters/ConditionBadge';
import { BackstoryPanel } from '../components/characters/BackstoryPanel';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Character, Condition, Backstory } from '../api/types';
import { ALL_CONDITIONS } from '../api/types';

function HpBar({ current, max }: { current: number; max: number }) {
  const pct = max > 0 ? (current / max) * 100 : 0;
  const cls = pct > 50 ? 'hp-high' : pct > 25 ? 'hp-mid' : 'hp-low';
  return (
    <div className="hp-bar-container hp-bar-large">
      <div className={`hp-bar-fill ${cls}`} style={{ width: `${Math.min(100, pct)}%` }} />
      <span className="hp-bar-label">{current} / {max} HP</span>
    </div>
  );
}

export function CharacterDetailPage() {
  const { campaignId, charId } = useParams<{ campaignId: string; charId: string }>();
  const { data: character, loading, error, refetch } = useApi<Character>(
    () => getCharacter(campaignId!, charId!),
    [campaignId, charId],
  );

  const [hpInput, setHpInput] = useState('');
  const [addCond, setAddCond] = useState<Condition>('Blinded');
  const [updating, setUpdating] = useState(false);

  const doUpdate = async (changes: Parameters<typeof updateCharacter>[2]) => {
    setUpdating(true);
    try {
      await updateCharacter(campaignId!, charId!, changes);
      refetch();
    } finally {
      setUpdating(false);
    }
  };

  const handleSetHp = async () => {
    if (!hpInput) return;
    await doUpdate({ current_hp: Number(hpInput) });
    setHpInput('');
  };

  const handleAddCondition = async () => {
    if (!character || character.conditions.includes(addCond)) return;
    await doUpdate({ conditions: [...character.conditions, addCond] });
  };

  const handleRemoveCondition = async (c: Condition) => {
    if (!character) return;
    await doUpdate({ conditions: character.conditions.filter((x) => x !== c) });
  };

  const handleAnalyzed = async (b: Backstory) => {
    void b;
    refetch();
  };

  if (loading) return <div className="page"><LoadingSpinner /></div>;
  if (error) return <div className="page"><ErrorBanner message={error} /></div>;
  if (!character) return null;

  const availableConditions = ALL_CONDITIONS.filter((c) => !character.conditions.includes(c));

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <Link to={`/campaigns/${campaignId}/characters`} className="breadcrumb">← Characters</Link>
          <h1>{character.name}</h1>
        </div>
        <div className="character-type-badges">
          <span className="badge badge-info">{character.character_type}</span>
          {character.class && <span className="badge">{character.class}</span>}
          {character.race && <span className="badge">{character.race}</span>}
          {!character.is_alive && <span className="badge badge-danger">Dead</span>}
        </div>
      </div>

      <div className="detail-grid">
        <div className="detail-main">
          <div className="stat-row">
            <div className="stat-box"><div className="stat-label">Level</div><div className="stat-value">{character.level}</div></div>
            <div className="stat-box"><div className="stat-label">AC</div><div className="stat-value">{character.armor_class}</div></div>
            <div className="stat-box"><div className="stat-label">Speed</div><div className="stat-value">{character.speed}ft</div></div>
          </div>

          <HpBar current={character.current_hp} max={character.max_hp} />

          <div className="inline-control">
            <input
              type="number"
              className="form-input form-input-num"
              placeholder="Set HP"
              value={hpInput}
              onChange={(e) => setHpInput(e.target.value)}
            />
            <button className="btn btn-sm" onClick={handleSetHp} disabled={!hpInput || updating}>Set HP</button>
          </div>

          {character.ability_scores && (
            <div className="ability-scores-display">
              <h3>Ability Scores</h3>
              <div className="ability-row">
                {Object.entries(character.ability_scores).map(([key, val]) => (
                  <div key={key} className="ability-box">
                    <div className="ability-label">{key.slice(0, 3).toUpperCase()}</div>
                    <div className="ability-value">{val}</div>
                    <div className="ability-mod">{val >= 10 ? '+' : ''}{Math.floor((val - 10) / 2)}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="conditions-section">
            <h3>Conditions</h3>
            <div className="condition-list">
              {character.conditions.length === 0 && <span className="empty-state">No active conditions</span>}
              {character.conditions.map((c) => (
                <ConditionBadge key={c} condition={c} onRemove={handleRemoveCondition} />
              ))}
            </div>
            {availableConditions.length > 0 && (
              <div className="inline-control">
                <select
                  className="form-input"
                  value={addCond}
                  onChange={(e) => setAddCond(e.target.value as Condition)}
                >
                  {availableConditions.map((c) => <option key={c} value={c}>{c}</option>)}
                </select>
                <button className="btn btn-sm" onClick={handleAddCondition} disabled={updating}>Add Condition</button>
              </div>
            )}
          </div>

          <div className="alive-toggle">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={character.is_alive}
                onChange={() => doUpdate({ is_alive: !character.is_alive })}
                disabled={updating}
              />
              <span>Is Alive</span>
            </label>
          </div>
        </div>

        <div className="detail-side">
          <BackstoryPanel
            campaignId={campaignId!}
            characterId={charId!}
            backstory={character.backstory}
            onAnalyzed={handleAnalyzed}
          />
        </div>
      </div>
    </div>
  );
}
