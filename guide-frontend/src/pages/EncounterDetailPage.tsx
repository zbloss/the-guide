import { useState, useEffect, useCallback } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getEncounter, startEncounter, nextTurn, endEncounter, getEncounterReplay } from '../api/encounters';
import { saveTemplate } from '../api/templates';
import { CombatTracker } from '../components/encounters/CombatTracker';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { EncounterSummary, EncounterTurnSnapshot } from '../api/types';
import { BASE_URL } from '../api/client';

export function EncounterDetailPage() {
  const { campaignId, encId } = useParams<{ campaignId: string; encId: string }>();
  const { data: initialEncounter, loading, error } = useApi<EncounterSummary>(
    () => getEncounter(campaignId!, encId!),
    [campaignId, encId],
  );

  // Local state so combat updates don't require full re-fetch
  const [encounter, setEncounter] = useState<EncounterSummary | null>(null);
  const [actionError, setActionError] = useState('');
  const [actionLoading, setActionLoading] = useState(false);
  const [templateSaved, setTemplateSaved] = useState(false);
  const [templateSaving, setTemplateSaving] = useState(false);

  // Replay state
  const [showReplay, setShowReplay] = useState(false);
  const [snapshots, setSnapshots] = useState<EncounterTurnSnapshot[]>([]);
  const [replayIndex, setReplayIndex] = useState(0);
  const [replayPlaying, setReplayPlaying] = useState(false);
  const [snapshotsLoading, setSnapshotsLoading] = useState(false);

  const displayed = encounter ?? initialEncounter;

  const doAction = useCallback(async (fn: () => Promise<EncounterSummary>) => {
    setActionError('');
    setActionLoading(true);
    try {
      const updated = await fn();
      setEncounter(updated);
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionLoading(false);
    }
  }, []);

  // Keyboard shortcut: Space → next turn when active
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.code === 'Space' && displayed?.status === 'active' && !actionLoading) {
        // Don't trigger when typing in an input/textarea
        const tag = (e.target as HTMLElement).tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
        e.preventDefault();
        doAction(() => nextTurn(campaignId!, encId!));
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [displayed?.status, actionLoading, campaignId, encId, doAction]);

  // Replay auto-play
  useEffect(() => {
    if (!replayPlaying) return;
    if (replayIndex >= snapshots.length - 1) { setReplayPlaying(false); return; }
    const t = setTimeout(() => setReplayIndex((i) => i + 1), 1500);
    return () => clearTimeout(t);
  }, [replayPlaying, replayIndex, snapshots.length]);

  const handleSaveAsTemplate = async () => {
    setTemplateSaving(true);
    setActionError('');
    try {
      await saveTemplate(campaignId!, encId!);
      setTemplateSaved(true);
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleDownloadLog = async () => {
    const res = await fetch(`${BASE_URL}/campaigns/${campaignId}/encounters/${encId}/log`);
    const text = await res.text();
    const blob = new Blob([text], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `combat-log.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleLoadReplay = async () => {
    setSnapshotsLoading(true);
    try {
      const snaps = await getEncounterReplay(campaignId!, encId!);
      setSnapshots(snaps);
      setReplayIndex(0);
      setShowReplay(true);
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setSnapshotsLoading(false);
    }
  };

  if (loading) return <div className="page"><LoadingSpinner /></div>;
  if (error) return <div className="page"><ErrorBanner message={error} /></div>;
  if (!displayed) return null;

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <Link to={`/campaigns/${campaignId}/encounters`} className="breadcrumb">← Encounters</Link>
          <h1>{displayed.name ?? 'Unnamed Encounter'}</h1>
        </div>
        <div className="encounter-actions">
          {displayed.status === 'pending' && (
            <button className="btn btn-primary" onClick={() => doAction(() => startEncounter(campaignId!, encId!))} disabled={actionLoading}>
              ⚔️ Start Combat
            </button>
          )}
          {displayed.status === 'active' && (
            <>
              <button className="btn btn-primary" onClick={() => doAction(() => nextTurn(campaignId!, encId!))} disabled={actionLoading}>
                ▶ Next Turn
              </button>
              <button className="btn btn-danger" onClick={() => doAction(() => endEncounter(campaignId!, encId!))} disabled={actionLoading}>
                End Encounter
              </button>
            </>
          )}
          {displayed.status === 'completed' && (
            <span className="badge badge-success">Encounter Completed</span>
          )}
          {(displayed.status === 'active' || displayed.status === 'completed') && (
            <button className="btn btn-secondary" onClick={handleLoadReplay} disabled={snapshotsLoading}>
              {snapshotsLoading ? 'Loading…' : 'Replay'}
            </button>
          )}
          {templateSaved ? (
            <span className="badge badge-success">Template saved</span>
          ) : (
            <button className="btn btn-secondary" onClick={handleSaveAsTemplate} disabled={templateSaving}>
              {templateSaving ? 'Saving…' : 'Save as Template'}
            </button>
          )}
          <button className="btn btn-secondary" onClick={handleDownloadLog}>
            Download Log
          </button>
        </div>
      </div>

      {actionError && <ErrorBanner message={actionError} />}

      {displayed.description && <p className="encounter-description">{displayed.description}</p>}

      {displayed.status === 'pending' && displayed.participants.length > 0 && (
        <div>
          <h3>Participants ({displayed.participants.length})</h3>
          <ul className="participant-preview">
            {displayed.participants.map((p) => (
              <li key={p.id}>{p.name} (AC {p.armor_class}, {p.max_hp} HP{p.initiative_total !== 0 ? `, Init ${p.initiative_total}` : ''})</li>
            ))}
          </ul>
          <p className="help-text">Start combat to auto-roll initiative and begin turn order.</p>
        </div>
      )}

      {(displayed.status === 'active' || displayed.status === 'completed') && (
        <CombatTracker
          encounter={displayed}
          campaignId={campaignId!}
          onUpdate={setEncounter}
        />
      )}

      {showReplay && snapshots.length > 0 && (
        <div className="replay-panel" style={{ marginTop: 24, border: '1px solid var(--color-border, #333)', borderRadius: 8, padding: 16 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <h3 style={{ margin: 0 }}>Encounter Replay</h3>
            <span style={{ color: '#aaa', fontSize: 13 }}>
              Turn {replayIndex + 1} of {snapshots.length} — Round {snapshots[replayIndex].round_number}
            </span>
            <button className="btn btn-sm" onClick={() => setShowReplay(false)} style={{ marginLeft: 'auto' }}>✕ Close</button>
          </div>
          <div style={{ display: 'flex', gap: 8, marginBottom: 12, alignItems: 'center' }}>
            <button className="btn btn-sm" onClick={() => { setReplayIndex(0); setReplayPlaying(false); }} disabled={replayIndex === 0}>|◀</button>
            <button className="btn btn-sm" onClick={() => setReplayIndex((i) => Math.max(0, i - 1))} disabled={replayIndex === 0}>◀</button>
            <button className="btn btn-sm btn-primary" onClick={() => setReplayPlaying((p) => !p)}>
              {replayPlaying ? 'Pause' : 'Play'}
            </button>
            <button className="btn btn-sm" onClick={() => setReplayIndex((i) => Math.min(snapshots.length - 1, i + 1))} disabled={replayIndex >= snapshots.length - 1}>▶</button>
            <button className="btn btn-sm" onClick={() => { setReplayIndex(snapshots.length - 1); setReplayPlaying(false); }} disabled={replayIndex >= snapshots.length - 1}>▶|</button>
            <input
              type="range" min={0} max={snapshots.length - 1} value={replayIndex}
              onChange={(e) => { setReplayPlaying(false); setReplayIndex(Number(e.target.value)); }}
              style={{ flex: 1 }}
            />
          </div>
          <CombatTracker
            encounter={snapshots[replayIndex].snapshot}
            campaignId={campaignId!}
            onUpdate={() => {}}
            readOnly
          />
        </div>
      )}
    </div>
  );
}
