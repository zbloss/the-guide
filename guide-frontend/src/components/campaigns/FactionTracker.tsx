import { useState } from 'react';
import { useApi } from '../../hooks/useApi';
import { listFactions, createFaction, updateFactionReputation, deleteFaction } from '../../api/factions';
import type { Faction, CreateFactionRequest, UpdateFactionReputationRequest } from '../../api/types';

const STANDINGS = ['Hostile', 'Unfriendly', 'Neutral', 'Friendly', 'Honored', 'Exalted'];

const standingColor: Record<string, string> = {
  Hostile: '#ef4444',
  Unfriendly: '#f97316',
  Neutral: '#6b7280',
  Friendly: '#3b82f6',
  Honored: '#22c55e',
  Exalted: '#a855f7',
};

function StandingBadge({ standing }: { standing: string }) {
  const color = standingColor[standing] ?? '#6b7280';
  return (
    <span style={{
      fontSize: 11, fontWeight: 700, padding: '2px 8px', borderRadius: 99,
      background: `${color}22`, color, border: `1px solid ${color}66`,
    }}>
      {standing}
    </span>
  );
}

interface Props {
  campaignId: string;
}

export function FactionTracker({ campaignId }: Props) {
  const { data: factions, loading, refetch } = useApi<Faction[]>(
    () => listFactions(campaignId),
    [campaignId],
  );
  const [open, setOpen] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const handleCreate = async () => {
    if (!name.trim()) return;
    setSaving(true);
    setError('');
    try {
      const req: CreateFactionRequest = { name: name.trim(), description: description.trim() || undefined };
      await createFaction(campaignId, req);
      setName('');
      setDescription('');
      setShowForm(false);
      refetch();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleStandingChange = async (faction: Faction, standing: string) => {
    const req: UpdateFactionReputationRequest = { standing, notes: faction.notes ?? undefined };
    await updateFactionReputation(campaignId, faction.id, req);
    refetch();
  };

  const handleDelete = async (factionId: string) => {
    await deleteFaction(campaignId, factionId);
    refetch();
  };

  if (loading && !factions) return null;
  const list = factions ?? [];

  return (
    <div className="backstory-panel" style={{ marginTop: 20 }}>
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', cursor: 'pointer' }}
        onClick={() => setOpen((o) => !o)}
      >
        <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-heading)' }}>
          Factions ({list.length})
        </span>
        <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>{open ? '▲' : '▼'}</span>
      </div>

      {open && (
        <div style={{ marginTop: 12 }}>
          {error && <div className="form-error-banner">{error}</div>}
          {list.length === 0 && <p style={{ fontSize: 13, color: 'var(--text-muted)' }}>No factions tracked yet.</p>}
          {list.map((faction) => (
            <div key={faction.id} style={{
              display: 'flex', alignItems: 'center', gap: 10, padding: '8px 0',
              borderBottom: '1px solid var(--border)',
            }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-heading)' }}>{faction.name}</div>
                {faction.description && <div style={{ fontSize: 12, color: 'var(--text-muted)' }}>{faction.description}</div>}
              </div>
              <select
                className="form-input"
                style={{ width: 120, fontSize: 12 }}
                value={faction.standing}
                onChange={(e) => handleStandingChange(faction, e.target.value)}
              >
                {STANDINGS.map((s) => <option key={s} value={s}>{s}</option>)}
              </select>
              <StandingBadge standing={faction.standing} />
              <button className="btn btn-sm" style={{ color: 'var(--color-danger)' }} onClick={() => handleDelete(faction.id)}>✕</button>
            </div>
          ))}

          {showForm ? (
            <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
              <input
                className="form-input"
                placeholder="Faction name"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <input
                className="form-input"
                placeholder="Description (optional)"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
              <div style={{ display: 'flex', gap: 8 }}>
                <button className="btn btn-primary btn-sm" onClick={handleCreate} disabled={saving || !name.trim()}>
                  {saving ? 'Saving…' : 'Add Faction'}
                </button>
                <button className="btn btn-sm" onClick={() => setShowForm(false)}>Cancel</button>
              </div>
            </div>
          ) : (
            <button className="btn btn-sm" style={{ marginTop: 10 }} onClick={() => setShowForm(true)}>
              + Add Faction
            </button>
          )}
        </div>
      )}
    </div>
  );
}
