import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  listArcs,
  updateArcNotes,
  updateArcStatus,
  listEvents,
  updateEventNotes,
  listSubplots,
  updateSubplotNotes,
  updateSubplotStatus,
  listCharacterArcs,
  updateCharacterArcNotes,
  listStoryEncounters,
  updateStoryEncounterNotes,
  activateEncounter,
} from '../api/story';
import { Badge } from '../components/common/Badge';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type {
  StoryArc,
  StoryEvent,
  StorySubplot,
  CharacterArc,
  PrepopulatedEncounter,
  ArcStatus,
} from '../api/types';

// ── helpers ──────────────────────────────────────────────────────────────────

function arcStatusVariant(status: ArcStatus) {
  if (status === 'resolved') return 'success' as const;
  if (status === 'abandoned') return 'danger' as const;
  return 'info' as const;
}

function significanceVariant(sig: string) {
  return sig === 'major' ? 'warning' as const : 'default' as const;
}

// ── Sub-components ────────────────────────────────────────────────────────────

interface NotesEditorProps {
  initialValue: string | null;
  onSave: (value: string | null) => Promise<void>;
}

function NotesEditor({ initialValue, onSave }: NotesEditorProps) {
  const [notes, setNotes] = useState(initialValue ?? '');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(notes.trim() || null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ marginTop: 8 }}>
      <textarea
        className="form-input"
        rows={2}
        placeholder="DM notes..."
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        style={{ width: '100%', resize: 'vertical', fontSize: 13 }}
      />
      <button
        className="btn btn-sm btn-primary"
        onClick={handleSave}
        disabled={saving}
        style={{ marginTop: 4 }}
      >
        {saving ? 'Saving…' : saved ? 'Saved!' : 'Save Notes'}
      </button>
    </div>
  );
}

// ── Arc Card + Events ─────────────────────────────────────────────────────────

interface ArcCardProps {
  arc: StoryArc;
  campaignId: string;
  onStatusChange: (arcId: string, status: string) => void;
}

function ArcCard({ arc, campaignId, onStatusChange }: ArcCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [events, setEvents] = useState<StoryEvent[] | null>(null);
  const [eventsLoading, setEventsLoading] = useState(false);

  const handleToggle = async () => {
    if (!expanded && events === null) {
      setEventsLoading(true);
      try {
        const data = await listEvents(campaignId, arc.id);
        setEvents(data);
      } finally {
        setEventsLoading(false);
      }
    }
    setExpanded((v) => !v);
  };

  const handleStatusChange = async (newStatus: string) => {
    await updateArcStatus(campaignId, arc.id, newStatus);
    onStatusChange(arc.id, newStatus);
  };

  return (
    <div
      className="card"
      style={{
        padding: '1rem',
        marginBottom: '0.75rem',
        border: '1px solid var(--color-border, #333)',
        borderRadius: '0.5rem',
        background: 'var(--color-surface, #1e1e2e)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        <button
          onClick={handleToggle}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            fontSize: 18,
            color: 'var(--color-text, #cdd6f4)',
            padding: 0,
          }}
        >
          {expanded ? '▼' : '▶'}
        </button>
        <span style={{ fontWeight: 600, fontSize: 15 }}>
          Arc {arc.arc_order}: {arc.title}
        </span>
        <Badge label={arc.status} variant={arcStatusVariant(arc.status)} />
        <select
          className="form-input"
          value={arc.status}
          onChange={(e) => handleStatusChange(e.target.value)}
          style={{ marginLeft: 'auto', width: 'auto', fontSize: 13, padding: '2px 6px' }}
        >
          <option value="open">Open</option>
          <option value="resolved">Resolved</option>
          <option value="abandoned">Abandoned</option>
        </select>
      </div>

      <p style={{ margin: '0.5rem 0 0', fontSize: 14, color: 'var(--color-text-muted, #a6adc8)' }}>
        {arc.description}
      </p>

      <NotesEditor
        initialValue={arc.dm_notes}
        onSave={(val) => updateArcNotes(campaignId, arc.id, val)}
      />

      {expanded && (
        <div style={{ marginTop: '0.75rem', paddingLeft: '1rem', borderLeft: '2px solid var(--color-border, #333)' }}>
          {eventsLoading && <LoadingSpinner />}
          {events && events.length === 0 && (
            <p className="empty-state" style={{ fontSize: 13 }}>No events for this arc.</p>
          )}
          {events && events.map((ev) => (
            <EventRow key={ev.id} event={ev} campaignId={campaignId} />
          ))}
        </div>
      )}
    </div>
  );
}

// ── Event Row ─────────────────────────────────────────────────────────────────

interface EventRowProps {
  event: StoryEvent;
  campaignId: string;
}

function EventRow({ event, campaignId }: EventRowProps) {
  return (
    <div
      style={{
        padding: '0.6rem 0.75rem',
        marginBottom: '0.5rem',
        background: 'var(--color-surface-2, #2a2a3e)',
        borderRadius: '0.375rem',
      }}
    >
      <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6 }}>
        <span style={{ fontWeight: 500, fontSize: 14 }}>
          {event.event_order}. {event.title}
        </span>
        <Badge label={event.event_type} variant="info" />
        <Badge label={event.significance} variant={significanceVariant(event.significance)} />
        {event.is_dm_only && <Badge label="DM Only" variant="danger" />}
        {event.location && (
          <span style={{ fontSize: 12, color: 'var(--color-text-muted, #a6adc8)' }}>
            @ {event.location}
          </span>
        )}
      </div>
      <p style={{ margin: '0.4rem 0 0', fontSize: 13, color: 'var(--color-text-muted, #a6adc8)' }}>
        {event.description}
      </p>
      <NotesEditor
        initialValue={event.dm_notes}
        onSave={(val) => updateEventNotes(campaignId, event.id, val)}
      />
    </div>
  );
}

// ── Subplots Tab ──────────────────────────────────────────────────────────────

function SubplotsTab({ campaignId }: { campaignId: string }) {
  const [subplots, setSubplots] = useState<StorySubplot[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    listSubplots(campaignId)
      .then(setSubplots)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [campaignId]);

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBanner message={error} />;
  if (!subplots || subplots.length === 0)
    return <p className="empty-state">No subplots extracted yet.</p>;

  return (
    <div>
      {subplots.map((sp) => (
        <div
          key={sp.id}
          className="card"
          style={{
            padding: '0.75rem 1rem',
            marginBottom: '0.625rem',
            border: '1px solid var(--color-border, #333)',
            borderRadius: '0.5rem',
            background: 'var(--color-surface, #1e1e2e)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <span style={{ fontWeight: 600, fontSize: 14 }}>{sp.title}</span>
            <Badge label={sp.status} variant={arcStatusVariant(sp.status as ArcStatus)} />
            <select
              className="form-input"
              value={sp.status}
              onChange={async (e) => {
                const newStatus = e.target.value;
                await updateSubplotStatus(campaignId, sp.id, newStatus);
                setSubplots((prev) =>
                  prev ? prev.map((s) => s.id === sp.id ? { ...s, status: newStatus as typeof sp.status } : s) : prev
                );
              }}
              style={{ marginLeft: 'auto', width: 'auto', fontSize: 13, padding: '2px 6px' }}
            >
              <option value="open">Open</option>
              <option value="resolved">Resolved</option>
              <option value="abandoned">Abandoned</option>
            </select>
          </div>
          <p style={{ margin: '0.4rem 0 0', fontSize: 13, color: 'var(--color-text-muted, #a6adc8)' }}>
            {sp.description}
          </p>
          <NotesEditor
            initialValue={sp.dm_notes}
            onSave={(val) => updateSubplotNotes(campaignId, sp.id, val)}
          />
        </div>
      ))}
    </div>
  );
}

// ── Character Arcs Tab ────────────────────────────────────────────────────────

function CharacterArcsTab({ campaignId }: { campaignId: string }) {
  const [arcs, setArcs] = useState<CharacterArc[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    listCharacterArcs(campaignId)
      .then(setArcs)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [campaignId]);

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBanner message={error} />;
  if (!arcs || arcs.length === 0)
    return <p className="empty-state">No character arcs extracted yet.</p>;

  return (
    <div>
      {arcs.map((ca) => (
        <div
          key={ca.id}
          className="card"
          style={{
            padding: '0.75rem 1rem',
            marginBottom: '0.625rem',
            border: '1px solid var(--color-border, #333)',
            borderRadius: '0.5rem',
            background: 'var(--color-surface, #1e1e2e)',
          }}
        >
          <span style={{ fontWeight: 600, fontSize: 14 }}>{ca.character_name}</span>
          <p style={{ margin: '0.4rem 0 0', fontSize: 13, color: 'var(--color-text-muted, #a6adc8)' }}>
            {ca.description}
          </p>
          {ca.arc_points.length > 0 && (
            <ul style={{ margin: '0.5rem 0 0', paddingLeft: '1.25rem', fontSize: 13 }}>
              {ca.arc_points
                .slice()
                .sort((a, b) => a.order - b.order)
                .map((pt, i) => (
                  <li key={i} style={{ color: 'var(--color-text-muted, #a6adc8)', marginBottom: 2 }}>
                    {pt.description}
                  </li>
                ))}
            </ul>
          )}
          <NotesEditor
            initialValue={ca.dm_notes}
            onSave={(val) => updateCharacterArcNotes(campaignId, ca.id, val)}
          />
        </div>
      ))}
    </div>
  );
}

// ── Encounters Tab ────────────────────────────────────────────────────────────

interface EncountersTabProps {
  campaignId: string;
  onActivated: (encounterId: string) => void;
}

function EncountersTab({ campaignId, onActivated }: EncountersTabProps) {
  const [encounters, setEncounters] = useState<PrepopulatedEncounter[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [activating, setActivating] = useState<string | null>(null);

  useEffect(() => {
    listStoryEncounters(campaignId)
      .then(setEncounters)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [campaignId]);

  const handleActivate = async (enc: PrepopulatedEncounter) => {
    setActivating(enc.id);
    try {
      const result = await activateEncounter(campaignId, enc.id);
      onActivated(result.encounter_id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActivating(null);
    }
  };

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBanner message={error} />;
  if (!encounters || encounters.length === 0)
    return <p className="empty-state">No pre-populated encounters extracted yet.</p>;

  return (
    <div>
      {encounters.map((enc) => (
        <div
          key={enc.id}
          className="card"
          style={{
            padding: '0.75rem 1rem',
            marginBottom: '0.625rem',
            border: '1px solid var(--color-border, #333)',
            borderRadius: '0.5rem',
            background: 'var(--color-surface, #1e1e2e)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <span style={{ fontWeight: 600, fontSize: 14 }}>{enc.name}</span>
            {enc.difficulty_hint && (
              <Badge label={enc.difficulty_hint} variant="warning" />
            )}
            {enc.location && (
              <span style={{ fontSize: 12, color: 'var(--color-text-muted, #a6adc8)' }}>
                @ {enc.location}
              </span>
            )}
            <button
              className="btn btn-sm btn-primary"
              onClick={() => handleActivate(enc)}
              disabled={activating === enc.id}
              style={{ marginLeft: 'auto' }}
            >
              {activating === enc.id ? 'Activating…' : 'Activate'}
            </button>
          </div>
          <p style={{ margin: '0.4rem 0 0', fontSize: 13, color: 'var(--color-text-muted, #a6adc8)' }}>
            {enc.description}
          </p>
          {enc.monsters.length > 0 && (
            <div style={{ marginTop: '0.4rem', fontSize: 13 }}>
              <strong>Monsters: </strong>
              {enc.monsters.map((m, i) => (
                <span key={i}>
                  {m.count != null ? `${m.count}x ` : ''}{m.name}
                  {m.cr != null ? ` (CR ${m.cr})` : ''}
                  {i < enc.monsters.length - 1 ? ', ' : ''}
                </span>
              ))}
            </div>
          )}
          <NotesEditor
            initialValue={enc.dm_notes}
            onSave={(val) => updateStoryEncounterNotes(campaignId, enc.id, val)}
          />
        </div>
      ))}
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

type BottomTab = 'subplots' | 'character_arcs' | 'encounters';

export function StoryPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const navigate = useNavigate();

  const [arcs, setArcs] = useState<StoryArc[] | null>(null);
  const [arcsLoading, setArcsLoading] = useState(true);
  const [arcsError, setArcsError] = useState('');
  const [bottomTab, setBottomTab] = useState<BottomTab>('subplots');

  useEffect(() => {
    if (!campaignId) return;
    listArcs(campaignId)
      .then(setArcs)
      .catch((e: unknown) => setArcsError(e instanceof Error ? e.message : String(e)))
      .finally(() => setArcsLoading(false));
  }, [campaignId]);

  const handleArcStatusChange = (arcId: string, status: string) => {
    setArcs((prev) =>
      prev
        ? prev.map((a) =>
            a.id === arcId ? { ...a, status: status as ArcStatus } : a
          )
        : prev
    );
  };

  const handleEncounterActivated = (encounterId: string) => {
    navigate(`/campaigns/${campaignId}/encounters/${encounterId}`);
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Story</h2>
      </div>

      {/* Story Arcs */}
      <div style={{ marginBottom: '1.5rem' }}>
        <h3 style={{ marginBottom: '0.75rem' }}>Story Arcs</h3>
        {arcsLoading && <LoadingSpinner />}
        {arcsError && <ErrorBanner message={arcsError} />}
        {arcs && arcs.length === 0 && (
          <p className="empty-state">
            No story arcs extracted yet. Ingest a campaign document to populate the story.
          </p>
        )}
        {arcs &&
          arcs
            .slice()
            .sort((a, b) => a.arc_order - b.arc_order)
            .map((arc) => (
              <ArcCard
                key={arc.id}
                arc={arc}
                campaignId={campaignId!}
                onStatusChange={handleArcStatusChange}
              />
            ))}
      </div>

      {/* Bottom Tabs */}
      <div className="tab-nav" style={{ marginBottom: '1rem' }}>
        <button
          className={`tab-link ${bottomTab === 'subplots' ? 'active' : ''}`}
          onClick={() => setBottomTab('subplots')}
        >
          Subplots
        </button>
        <button
          className={`tab-link ${bottomTab === 'character_arcs' ? 'active' : ''}`}
          onClick={() => setBottomTab('character_arcs')}
        >
          Character Arcs
        </button>
        <button
          className={`tab-link ${bottomTab === 'encounters' ? 'active' : ''}`}
          onClick={() => setBottomTab('encounters')}
        >
          Pre-populated Encounters
        </button>
      </div>

      {bottomTab === 'subplots' && <SubplotsTab campaignId={campaignId!} />}
      {bottomTab === 'character_arcs' && <CharacterArcsTab campaignId={campaignId!} />}
      {bottomTab === 'encounters' && (
        <EncountersTab campaignId={campaignId!} onActivated={handleEncounterActivated} />
      )}
    </div>
  );
}
