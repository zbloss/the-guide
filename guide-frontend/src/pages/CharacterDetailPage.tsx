import { useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getCharacter, updateCharacter, deleteCharacter } from '../api/characters';
import { ConditionBadge } from '../components/characters/ConditionBadge';
import { BackstoryPanel } from '../components/characters/BackstoryPanel';
import { ConfirmButton } from '../components/common/ConfirmButton';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Character, Condition, Backstory, AbilityScores } from '../api/types';
import { ALL_CONDITIONS } from '../api/types';

function HpBar({ current, max }: { current: number; max: number }) {
  const pct = max > 0 ? (current / max) * 100 : 0;
  const overflow = current > max;
  const cls = overflow ? 'hp-overflow' : pct > 50 ? 'hp-high' : pct > 25 ? 'hp-mid' : 'hp-low';
  const label = overflow
    ? `${current} / ${max} HP (+${current - max})`
    : `${current} / ${max} HP`;
  return (
    <div className="hp-bar-container hp-bar-large">
      <div className={`hp-bar-fill ${cls}`} style={{ width: `${Math.min(100, pct)}%` }} />
      <span className="hp-bar-label">{label}</span>
    </div>
  );
}

function EditStatsForm({ character, onSave, onCancel }: { character: Character; onSave: () => void; onCancel: () => void }) {
  const { campaignId } = useParams<{ campaignId: string }>();
  const [form, setForm] = useState({
    name: character.name,
    class: character.class ?? '',
    race: character.race ?? '',
    level: character.level,
    max_hp: character.max_hp,
    armor_class: character.armor_class,
    speed: character.speed,
    str: character.ability_scores?.strength ?? 10,
    dex: character.ability_scores?.dexterity ?? 10,
    con: character.ability_scores?.constitution ?? 10,
    int: character.ability_scores?.intelligence ?? 10,
    wis: character.ability_scores?.wisdom ?? 10,
    cha: character.ability_scores?.charisma ?? 10,
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const numField = (key: keyof typeof form, label: string) => (
    <div className="form-field">
      <label className="form-label">{label}</label>
      <input type="number" className="form-input" value={form[key]} onChange={(e) => setForm(f => ({ ...f, [key]: Number(e.target.value) }))} />
    </div>
  );

  const handleSubmit = async () => {
    setSaving(true);
    setError('');
    try {
      const ability_scores: AbilityScores = {
        strength: form.str, dexterity: form.dex, constitution: form.con,
        intelligence: form.int, wisdom: form.wis, charisma: form.cha,
      };
      await updateCharacter(campaignId!, character.id, {
        name: form.name,
        class: form.class || undefined,
        race: form.race || undefined,
        level: form.level,
        max_hp: form.max_hp,
        armor_class: form.armor_class,
        speed: form.speed,
        ability_scores,
      });
      onSave();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="edit-stats-form">
      {error && <div className="form-error-banner">{error}</div>}
      <div className="form-row">
        <div className="form-field" style={{ flex: 2 }}>
          <label className="form-label">Name</label>
          <input className="form-input" value={form.name} onChange={(e) => setForm(f => ({ ...f, name: e.target.value }))} />
        </div>
        <div className="form-field">
          <label className="form-label">Class</label>
          <input className="form-input" value={form.class} onChange={(e) => setForm(f => ({ ...f, class: e.target.value }))} placeholder="Fighter" />
        </div>
        <div className="form-field">
          <label className="form-label">Race</label>
          <input className="form-input" value={form.race} onChange={(e) => setForm(f => ({ ...f, race: e.target.value }))} placeholder="Human" />
        </div>
      </div>
      <div className="form-row">
        {numField('level', 'Level')}
        {numField('max_hp', 'Max HP')}
        {numField('armor_class', 'AC')}
        {numField('speed', 'Speed')}
      </div>
      <div className="form-section-label" style={{ marginTop: 12 }}>Ability Scores</div>
      <div className="form-row" style={{ marginTop: 8 }}>
        {numField('str', 'STR')}
        {numField('dex', 'DEX')}
        {numField('con', 'CON')}
        {numField('int', 'INT')}
        {numField('wis', 'WIS')}
        {numField('cha', 'CHA')}
      </div>
      <div className="form-actions" style={{ marginTop: 12 }}>
        <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>{saving ? 'Saving…' : 'Save Changes'}</button>
        <button className="btn" onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}

export function CharacterDetailPage() {
  const { campaignId, charId } = useParams<{ campaignId: string; charId: string }>();
  const navigate = useNavigate();
  const { data: character, loading, error, refetch } = useApi<Character>(
    () => getCharacter(campaignId!, charId!),
    [campaignId, charId],
  );

  const [hpInput, setHpInput] = useState('');
  const [addCond, setAddCond] = useState<Condition>('blinded');
  const [updating, setUpdating] = useState(false);
  const [editingStats, setEditingStats] = useState(false);
  const [updateError, setUpdateError] = useState('');

  const doUpdate = async (changes: Parameters<typeof updateCharacter>[2]) => {
    setUpdateError('');
    setUpdating(true);
    try {
      await updateCharacter(campaignId!, charId!, changes);
      refetch();
    } catch (err: unknown) {
      setUpdateError(err instanceof Error ? err.message : String(err));
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

  const handleDelete = async () => {
    try {
      await deleteCharacter(campaignId!, charId!);
      navigate(`/campaigns/${campaignId}/characters`);
    } catch (err: unknown) {
      setUpdateError(err instanceof Error ? err.message : String(err));
    }
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
          <button className="btn btn-sm" onClick={() => setEditingStats(!editingStats)}>
            {editingStats ? 'Cancel Edit' : '✏️ Edit Stats'}
          </button>
          <ConfirmButton label="Delete Character" variant="danger" onConfirm={handleDelete} />
        </div>
      </div>

      {updateError && <ErrorBanner message={updateError} />}

      {editingStats ? (
        <EditStatsForm
          character={character}
          onSave={() => { setEditingStats(false); refetch(); }}
          onCancel={() => setEditingStats(false)}
        />
      ) : (
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
                placeholder="HP"
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
              onSaved={refetch}
            />
          </div>
        </div>
      )}
    </div>
  );
}
