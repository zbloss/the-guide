import { useState } from 'react';
import { Badge } from '../common/Badge';
import { ErrorBanner } from '../common/ErrorBanner';
import { updateEventNotes } from '../../api/story';
import type { StoryEvent, StoryEventType, StorySignificance } from '../../api/types';

function eventTypeBadgeVariant(t: StoryEventType): 'default' | 'info' | 'warning' | 'danger' | 'success' {
  switch (t) {
    case 'combat': return 'danger';
    case 'social': return 'info';
    case 'revelation': return 'warning';
    case 'travel': return 'default';
    case 'rest': return 'success';
    default: return 'default';
  }
}

function significanceBadgeVariant(s: StorySignificance): 'default' | 'warning' {
  return s === 'major' ? 'warning' : 'default';
}

interface StoryEventRowProps {
  event: StoryEvent;
}

export function StoryEventRow({ event }: StoryEventRowProps) {
  const [expanded, setExpanded] = useState(false);
  const [notes, setNotes] = useState(event.dm_notes ?? '');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState('');
  const [saved, setSaved] = useState(false);

  const handleSaveNotes = async () => {
    setSaving(true);
    setSaveError('');
    try {
      await updateEventNotes(event.campaign_id, event.id, notes || null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="story-event-row" style={{ borderLeft: '3px solid var(--color-border, #333)', paddingLeft: '0.75rem', marginBottom: '0.5rem' }}>
      <div
        className="story-event-header"
        style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', padding: '0.4rem 0' }}
        onClick={() => setExpanded((v) => !v)}
      >
        <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)', minWidth: '1.5rem' }}>
          {expanded ? '▼' : '▶'}
        </span>
        <span style={{ fontWeight: 500, flex: 1 }}>{event.title}</span>
        <Badge label={event.event_type} variant={eventTypeBadgeVariant(event.event_type)} />
        <Badge label={event.significance} variant={significanceBadgeVariant(event.significance)} />
        {event.location && (
          <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)' }}>
            {event.location}
          </span>
        )}
      </div>

      {expanded && (
        <div style={{ paddingTop: '0.5rem', paddingBottom: '0.5rem' }}>
          <p style={{ margin: '0 0 0.5rem', color: 'var(--color-text-muted, #aaa)', fontSize: '0.875rem' }}>
            {event.description}
          </p>

          {event.involved_characters.length > 0 && (
            <div style={{ marginBottom: '0.5rem' }}>
              <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)' }}>
                Characters: {event.involved_characters.join(', ')}
              </span>
            </div>
          )}

          <div style={{ marginTop: '0.5rem' }}>
            <label style={{ fontSize: '0.75rem', color: 'var(--color-text-muted, #888)', display: 'block', marginBottom: '0.25rem' }}>
              DM Notes
            </label>
            <textarea
              className="form-input"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={2}
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
        </div>
      )}
    </div>
  );
}
