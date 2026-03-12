import { useState, useCallback } from 'react';
import { createPlotHook, updatePlotHook, deletePlotHook } from '../../api/plotHooks';
import type { TrackedPlotHook, PlotHookStatus } from '../../api/types';

const STATUS_ORDER: PlotHookStatus[] = ['open', 'active', 'resolved'];

const STATUS_LABELS: Record<PlotHookStatus, string> = {
  open: 'Open',
  active: 'Active',
  resolved: 'Resolved',
};

interface PlotHookBoardProps {
  campaignId: string;
  characterId: string;
  hooks: TrackedPlotHook[];
  onChanged: () => void;
}

interface HookCardProps {
  campaignId: string;
  characterId: string;
  hook: TrackedPlotHook;
  onChanged: () => void;
}

function HookCard({ campaignId, characterId, hook, onChanged }: HookCardProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const currentIdx = STATUS_ORDER.indexOf(hook.status);
  const canGoLeft = currentIdx > 0;
  const canGoRight = currentIdx < STATUS_ORDER.length - 1;

  const moveStatus = async (direction: 'left' | 'right') => {
    const newStatus = direction === 'left'
      ? STATUS_ORDER[currentIdx - 1]
      : STATUS_ORDER[currentIdx + 1];
    setBusy(true);
    setError('');
    try {
      await updatePlotHook(campaignId, characterId, hook.id, { status: newStatus });
      onChanged();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDelete = async () => {
    setBusy(true);
    setError('');
    try {
      await deletePlotHook(campaignId, characterId, hook.id);
      onChanged();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="plot-hook-card card" style={{ marginBottom: 8, padding: '10px 12px' }}>
      {error && <div className="form-error-banner" style={{ marginBottom: 6, fontSize: 12 }}>{error}</div>}
      <p style={{ margin: '0 0 8px', fontSize: 13, lineHeight: 1.4 }}>{hook.hook_text}</p>
      <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
        {canGoLeft && (
          <button
            className="btn btn-sm"
            onClick={() => void moveStatus('left')}
            disabled={busy}
            title={`Move to ${STATUS_LABELS[STATUS_ORDER[currentIdx - 1]]}`}
            style={{ fontSize: 11 }}
          >
            ← {STATUS_LABELS[STATUS_ORDER[currentIdx - 1]]}
          </button>
        )}
        {canGoRight && (
          <button
            className="btn btn-sm btn-primary"
            onClick={() => void moveStatus('right')}
            disabled={busy}
            title={`Move to ${STATUS_LABELS[STATUS_ORDER[currentIdx + 1]]}`}
            style={{ fontSize: 11 }}
          >
            {STATUS_LABELS[STATUS_ORDER[currentIdx + 1]]} →
          </button>
        )}
        <button
          className="btn btn-sm btn-danger"
          onClick={() => void handleDelete()}
          disabled={busy}
          style={{ marginLeft: 'auto', fontSize: 11 }}
        >
          Delete
        </button>
      </div>
    </div>
  );
}

export function PlotHookBoard({ campaignId, characterId, hooks, onChanged }: PlotHookBoardProps) {
  const [showForm, setShowForm] = useState(false);
  const [newHookText, setNewHookText] = useState('');
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const [expanded, setExpanded] = useState(true);

  const hooksByStatus = useCallback(
    (status: PlotHookStatus) => hooks.filter((h) => h.status === status),
    [hooks],
  );

  const handleCreate = async () => {
    if (!newHookText.trim()) {
      setFormError('Hook text is required.');
      return;
    }
    setSaving(true);
    setFormError('');
    try {
      await createPlotHook(campaignId, characterId, {
        hook_text: newHookText.trim(),
        status: 'open',
      });
      setNewHookText('');
      setShowForm(false);
      onChanged();
    } catch (e: unknown) {
      setFormError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="plot-hook-board" style={{ marginTop: 20 }}>
      <div className="section-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
        <h3
          style={{ cursor: 'pointer', userSelect: 'none', margin: 0 }}
          onClick={() => setExpanded((e) => !e)}
        >
          {expanded ? '▼' : '▶'} Plot Hook Tracker{' '}
          {hooks.length > 0 && (
            <span className="badge badge-info">{hooks.length}</span>
          )}
        </h3>
        <button
          className="btn btn-sm btn-primary"
          onClick={() => { setExpanded(true); setShowForm((s) => !s); }}
        >
          {showForm ? 'Cancel' : '+ Add Hook'}
        </button>
      </div>

      {showForm && (
        <div className="card" style={{ marginBottom: 12, padding: '10px 12px' }}>
          {formError && <div className="form-error-banner" style={{ marginBottom: 6 }}>{formError}</div>}
          <div className="form-field">
            <label className="form-label">Hook Text</label>
            <textarea
              className="form-input"
              value={newHookText}
              onChange={(e) => setNewHookText(e.target.value)}
              placeholder="Describe the plot hook..."
              rows={3}
            />
          </div>
          <div className="form-actions">
            <button className="btn btn-primary btn-sm" onClick={() => void handleCreate()} disabled={saving}>
              {saving ? 'Adding…' : 'Add Hook'}
            </button>
            <button className="btn btn-sm" onClick={() => setShowForm(false)} disabled={saving}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {expanded && (
        <div
          className="plot-hook-columns"
          style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}
        >
          {STATUS_ORDER.map((status) => {
            const columnHooks = hooksByStatus(status);
            return (
              <div key={status} className="plot-hook-column">
                <div
                  style={{
                    fontSize: 12,
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    color: 'var(--text-muted)',
                    marginBottom: 8,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  {STATUS_LABELS[status]}
                  <span className="badge badge-info">{columnHooks.length}</span>
                </div>
                <div
                  style={{
                    minHeight: 80,
                    background: 'rgba(255,255,255,0.03)',
                    borderRadius: 6,
                    padding: columnHooks.length === 0 ? '12px 8px' : 0,
                  }}
                >
                  {columnHooks.length === 0 ? (
                    <p className="empty-state" style={{ fontSize: 12, margin: 0, textAlign: 'center' }}>
                      No hooks
                    </p>
                  ) : (
                    columnHooks.map((hook) => (
                      <HookCard
                        key={hook.id}
                        campaignId={campaignId}
                        characterId={characterId}
                        hook={hook}
                        onChanged={onChanged}
                      />
                    ))
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
