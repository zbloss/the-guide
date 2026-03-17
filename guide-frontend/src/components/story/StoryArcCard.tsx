import { useState } from 'react';
import { Badge } from '../common/Badge';
import { ErrorBanner } from '../common/ErrorBanner';
import { StoryEventRow } from './StoryEventRow';
import { updateArcNotes, updateArcStatus } from '../../api/story';
import type { StoryArc, StoryEvent, ArcStatus } from '../../api/types';

function arcStatusVariant(status: ArcStatus): 'default' | 'success' | 'danger' | 'warning' {
  switch (status) {
    case 'open': return 'warning';
    case 'resolved': return 'success';
    case 'abandoned': return 'danger';
    default: return 'default';
  }
}

interface StoryArcCardProps {
  arc: StoryArc;
  events: StoryEvent[];
}

export function StoryArcCard({ arc, events }: StoryArcCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [notes, setNotes] = useState(arc.dm_notes ?? '');
  const [status, setStatus] = useState<ArcStatus>(arc.status);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState('');
  const [saved, setSaved] = useState(false);
  const [statusSaving, setStatusSaving] = useState(false);

  const handleSaveNotes = async () => {
    setSaving(true);
    setSaveError('');
    try {
      await updateArcNotes(arc.campaign_id, arc.id, notes || null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleStatusChange = async (newStatus: ArcStatus) => {
    setStatusSaving(true);
    try {
      await updateArcStatus(arc.campaign_id, arc.id, newStatus);
      setStatus(newStatus);
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e.message : String(e));
    } finally {
      setStatusSaving(false);
    }
  };

  return (
    <div
      className="story-arc-card"
      style={{
        border: '1px solid var(--color-border, #333)',
        borderRadius: '0.5rem',
        marginBottom: '1rem',
        background: 'var(--color-surface, #1e1e2e)',
      }}
    >
      <div
        style={{ padding: '0.75rem 1rem', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.5rem' }}
        onClick={() => setExpanded((v) => !v)}
      >
        <span style={{ fontSize: '0.8rem', color: 'var(--color-text-muted, #888)', minWidth: '1.5rem' }}>
          {expanded ? '▼' : '▶'}
        </span>
        <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)', minWidth: '2rem' }}>
          #{arc.arc_order}
        </span>
        <span style={{ fontWeight: 600, flex: 1 }}>{arc.title}</span>
        <Badge label={status} variant={arcStatusVariant(status)} />
        <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)' }}>
          {events.length} event{events.length !== 1 ? 's' : ''}
        </span>
      </div>

      {expanded && (
        <div style={{ padding: '0 1rem 1rem' }}>
          <p style={{ color: 'var(--color-text-muted, #aaa)', fontSize: '0.875rem', marginTop: 0 }}>
            {arc.description}
          </p>

          <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginBottom: '0.75rem' }}>
            <label style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)' }}>Status:</label>
            <select
              className="form-input"
              style={{ width: 'auto', fontSize: '0.875rem' }}
              value={status}
              onChange={(e) => handleStatusChange(e.target.value as ArcStatus)}
              disabled={statusSaving}
            >
              <option value="open">Open</option>
              <option value="resolved">Resolved</option>
              <option value="abandoned">Abandoned</option>
            </select>
          </div>

          <div style={{ marginBottom: '0.75rem' }}>
            <label style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)', display: 'block', marginBottom: '0.25rem' }}>
              DM Notes
            </label>
            <textarea
              className="form-input"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
              placeholder="Private DM notes..."
              style={{ width: '100%', fontSize: '0.875rem' }}
            />
            {saveError && <ErrorBanner message={saveError} />}
            <button
              className="btn btn-sm btn-primary"
              onClick={handleSaveNotes}
              disabled={saving}
              style={{ marginTop: '0.25rem' }}
            >
              {saving ? 'Saving...' : saved ? 'Saved!' : 'Save Notes'}
            </button>
          </div>

          {events.length > 0 && (
            <div>
              <h4 style={{ fontSize: '0.875rem', marginBottom: '0.5rem', color: 'var(--color-text-muted, #ccc)' }}>
                Events
              </h4>
              {events
                .slice()
                .sort((a, b) => a.event_order - b.event_order)
                .map((evt) => (
                  <StoryEventRow key={evt.id} event={evt} />
                ))}
            </div>
          )}

          {events.length === 0 && (
            <p className="empty-state" style={{ fontSize: '0.875rem' }}>No events linked to this arc.</p>
          )}
        </div>
      )}
    </div>
  );
}
