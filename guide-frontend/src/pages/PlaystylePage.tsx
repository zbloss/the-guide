import { useState, useEffect } from 'react';
import type { PlaystyleProfile, Pacing, Lethality, TonePreference, CombatComplexity } from '../api/types';

const STORAGE_KEY = 'playstyle_profile';

const DEFAULT_PROFILE: PlaystyleProfile = {
  pacing: 'moderate',
  lethality: 'moderate',
  tone: 'balanced',
  combat_complexity: 'moderate',
  roleplay_focus: 5,
  exploration_focus: 5,
  custom_notes: '',
};

function loadProfile(): PlaystyleProfile {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_PROFILE };
    return { ...DEFAULT_PROFILE, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_PROFILE };
  }
}

function RadioGroup<T extends string>({
  label, value, options, onChange,
}: { label: string; value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="form-field">
      <div className="form-label">{label}</div>
      <div className="radio-group-row">
        {options.map((opt) => (
          <label key={opt.value} className="radio-label">
            <input type="radio" name={label} value={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} />
            {opt.label}
          </label>
        ))}
      </div>
    </div>
  );
}

function SliderField({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <div className="form-field">
      <div className="form-label">{label} <span className="slider-value">{value}/10</span></div>
      <input
        type="range"
        min={1}
        max={10}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="range-input"
      />
    </div>
  );
}

export function PlaystylePage() {
  const [profile, setProfile] = useState<PlaystyleProfile>(loadProfile);
  const [saved, setSaved] = useState(false);

  useEffect(() => { setSaved(false); }, [profile]);

  const handleSave = () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(profile));
    setSaved(true);
  };

  const handleReset = () => {
    setProfile({ ...DEFAULT_PROFILE });
    localStorage.removeItem(STORAGE_KEY);
  };

  const set = <K extends keyof PlaystyleProfile>(key: K, val: PlaystyleProfile[K]) =>
    setProfile((p) => ({ ...p, [key]: val }));

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1>Playstyle Profile</h1>
          <p className="page-subtitle">Configure your DM preferences to personalize AI responses and encounter generation.</p>
        </div>
        <div className="btn-group">
          <button className="btn btn-primary" onClick={handleSave}>{saved ? '✓ Saved' : 'Save Profile'}</button>
          <button className="btn" onClick={handleReset}>Reset to Defaults</button>
        </div>
      </div>

      <div className="playstyle-grid">
        <div className="card">
          <h3 className="card-section-title">Campaign Pacing</h3>
          <RadioGroup<Pacing>
            label="Session Pacing"
            value={profile.pacing}
            onChange={(v) => set('pacing', v)}
            options={[
              { value: 'fast', label: 'Fast — action-packed, quick scenes' },
              { value: 'moderate', label: 'Moderate — balanced exploration and action' },
              { value: 'slow', label: 'Slow — deep world-building and roleplay' },
            ]}
          />
        </div>

        <div className="card">
          <h3 className="card-section-title">Lethality</h3>
          <RadioGroup<Lethality>
            label="Combat Lethality"
            value={profile.lethality}
            onChange={(v) => set('lethality', v)}
            options={[
              { value: 'lethal', label: 'Lethal — death is always on the table' },
              { value: 'moderate', label: 'Moderate — consequences without TPK focus' },
              { value: 'forgiving', label: 'Forgiving — heroic survival, narrative focus' },
            ]}
          />
        </div>

        <div className="card">
          <h3 className="card-section-title">Tone</h3>
          <RadioGroup<TonePreference>
            label="Campaign Tone"
            value={profile.tone}
            onChange={(v) => set('tone', v)}
            options={[
              { value: 'dark', label: 'Dark — grim, moral ambiguity' },
              { value: 'balanced', label: 'Balanced — mix of light and shadow' },
              { value: 'heroic', label: 'Heroic — epic, triumphant' },
            ]}
          />
        </div>

        <div className="card">
          <h3 className="card-section-title">Combat Complexity</h3>
          <RadioGroup<CombatComplexity>
            label="Combat Style"
            value={profile.combat_complexity}
            onChange={(v) => set('combat_complexity', v)}
            options={[
              { value: 'high', label: 'High — tactical, positioning matters' },
              { value: 'moderate', label: 'Moderate — standard 5e combat' },
              { value: 'simple', label: 'Simple — narrative-driven, fast resolution' },
            ]}
          />
        </div>

        <div className="card">
          <h3 className="card-section-title">Focus Balance</h3>
          <SliderField label="Roleplay Focus" value={profile.roleplay_focus} onChange={(v) => set('roleplay_focus', v)} />
          <SliderField label="Exploration Focus" value={profile.exploration_focus} onChange={(v) => set('exploration_focus', v)} />
        </div>

        <div className="card">
          <h3 className="card-section-title">Custom Notes</h3>
          <div className="form-field">
            <div className="form-label">Additional DM Preferences</div>
            <textarea
              className="form-input backstory-textarea"
              value={profile.custom_notes}
              onChange={(e) => set('custom_notes', e.target.value)}
              placeholder="Anything else the AI should know about your campaign style…"
              rows={5}
            />
          </div>
        </div>
      </div>

      <div className="playstyle-summary">
        <h3>Current Profile</h3>
        <div className="playstyle-chips">
          <span className="badge badge-info">{profile.pacing} pacing</span>
          <span className="badge badge-warning">{profile.lethality} lethality</span>
          <span className="badge">{profile.tone} tone</span>
          <span className="badge badge-default">{profile.combat_complexity} combat</span>
          <span className="badge badge-default">roleplay {profile.roleplay_focus}/10</span>
          <span className="badge badge-default">exploration {profile.exploration_focus}/10</span>
        </div>
      </div>
    </div>
  );
}
