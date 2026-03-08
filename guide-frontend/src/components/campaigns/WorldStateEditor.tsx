import { useState, useEffect } from 'react';
import type { Campaign, WorldState } from '../../api/types';
import { updateCampaign } from '../../api/campaigns';

interface WorldStateEditorProps {
  campaign: Campaign;
  onSaved: (updated: Campaign) => void;
}

function TagList({
  label,
  tags,
  onChange,
}: {
  label: string;
  tags: string[];
  onChange: (tags: string[]) => void;
}) {
  const [input, setInput] = useState('');

  const add = () => {
    const v = input.trim();
    if (v && !tags.includes(v)) {
      onChange([...tags, v]);
      setInput('');
    }
  };

  return (
    <div className="form-field">
      <label className="form-label">{label}</label>
      <div className="tag-list">
        {tags.map((t) => (
          <span key={t} className="tag">
            {t}
            <button className="tag-remove" onClick={() => onChange(tags.filter((x) => x !== t))} aria-label={`Remove ${t}`}>
              ×
            </button>
          </span>
        ))}
      </div>
      <div className="tag-input-row">
        <input
          className="form-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); add(); } }}
          placeholder={`Add ${label.toLowerCase()}…`}
        />
        <button type="button" className="btn btn-sm" onClick={add}>Add</button>
      </div>
    </div>
  );
}

export function WorldStateEditor({ campaign, onSaved }: WorldStateEditorProps) {
  const ws = campaign.world_state;
  const [location, setLocation] = useState(ws?.current_location ?? '');
  const [worldDate, setWorldDate] = useState(ws?.current_date_in_world ?? '');
  const [activeQuests, setActiveQuests] = useState<string[]>(ws?.active_quests ?? []);
  const [completedQuests, setCompletedQuests] = useState<string[]>(ws?.completed_quests ?? []);
  const [notes, setNotes] = useState(ws?.custom_notes ?? '');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    const w = campaign.world_state;
    setLocation(w?.current_location ?? '');
    setWorldDate(w?.current_date_in_world ?? '');
    setActiveQuests(w?.active_quests ?? []);
    setCompletedQuests(w?.completed_quests ?? []);
    setNotes(w?.custom_notes ?? '');
  }, [campaign]);

  const handleSave = async () => {
    setSaving(true);
    setError('');
    setSaved(false);
    try {
      const worldState: WorldState = {
        current_location: location.trim() || null,
        current_date_in_world: worldDate.trim() || null,
        active_quests: activeQuests,
        completed_quests: completedQuests,
        custom_notes: notes.trim() || null,
      };
      const updated = await updateCampaign(campaign.id, { world_state: worldState });
      onSaved(updated);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="world-state-editor">
      <h3>World State</h3>
      {error && <div className="form-error-banner">{error}</div>}

      <div className="form-field">
        <label className="form-label">Current Location</label>
        <input className="form-input" value={location} onChange={(e) => setLocation(e.target.value)} placeholder="e.g., Neverwinter" />
      </div>

      <div className="form-field">
        <label className="form-label">In-World Date</label>
        <input className="form-input" value={worldDate} onChange={(e) => setWorldDate(e.target.value)} placeholder="e.g., 15th of Mirtul, 1492 DR" />
      </div>

      <TagList label="Active Quests" tags={activeQuests} onChange={setActiveQuests} />
      <TagList label="Completed Quests" tags={completedQuests} onChange={setCompletedQuests} />

      <div className="form-field">
        <label className="form-label">Custom Notes</label>
        <textarea className="form-input" value={notes} onChange={(e) => setNotes(e.target.value)} rows={4} placeholder="Session notes, reminders…" />
      </div>

      <div className="form-actions">
        <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? 'Saving…' : saved ? 'Saved ✓' : 'Save World State'}
        </button>
      </div>
    </div>
  );
}
