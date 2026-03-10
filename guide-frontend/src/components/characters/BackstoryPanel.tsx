import { useState } from 'react';
import { analyzeBackstory, updateCharacter } from '../../api/characters';
import type { Backstory } from '../../api/types';
import { LoadingSpinner } from '../common/LoadingSpinner';

interface BackstoryPanelProps {
  campaignId: string;
  characterId: string;
  backstory: Backstory | null;
  onAnalyzed: (b: Backstory) => void;
}

export function BackstoryPanel({ campaignId, characterId, backstory, onAnalyzed }: BackstoryPanelProps) {
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState('');
  const [editingText, setEditingText] = useState(false);
  const [backstoryText, setBackstoryText] = useState(backstory?.raw_text ?? '');
  const [saving, setSaving] = useState(false);

  const handleSaveText = async () => {
    setSaving(true);
    setError('');
    try {
      await updateCharacter(campaignId, characterId, { backstory_text: backstoryText });
      setEditingText(false);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleAnalyze = async () => {
    const currentText = backstoryText.trim() || backstory?.raw_text?.trim();
    if (!currentText) {
      setError('Enter backstory text first, then click Save before analyzing.');
      return;
    }
    setAnalyzing(true);
    setError('');
    try {
      const result = await analyzeBackstory(campaignId, characterId);
      onAnalyzed(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setAnalyzing(false);
    }
  };

  const displayText = backstory?.raw_text ?? '';

  return (
    <div className="backstory-panel">
      <div className="section-header">
        <h3>Backstory</h3>
        <div className="btn-group">
          <button className="btn btn-sm" onClick={() => { setEditingText(!editingText); setBackstoryText(displayText); }}>
            {editingText ? 'Cancel' : 'Edit Text'}
          </button>
          <button
            className="btn btn-sm btn-primary"
            onClick={handleAnalyze}
            disabled={analyzing || (!backstoryText.trim() && !displayText.trim())}
            title={!backstoryText.trim() && !displayText.trim() ? 'Enter backstory text first' : 'Analyze with AI'}
          >
            {analyzing ? <><LoadingSpinner size={14} /> Analyzing…</> : 'Analyze with AI'}
          </button>
        </div>
      </div>

      {error && <div className="form-error-banner">{error}</div>}

      {editingText ? (
        <div className="backstory-edit">
          <textarea
            className="form-input backstory-textarea"
            value={backstoryText}
            onChange={(e) => setBackstoryText(e.target.value)}
            placeholder="Enter character backstory here…"
            rows={8}
          />
          <div className="form-actions">
            <button className="btn btn-sm btn-primary" onClick={handleSaveText} disabled={saving}>
              {saving ? 'Saving…' : 'Save Text'}
            </button>
            <button className="btn btn-sm" onClick={() => setEditingText(false)}>Cancel</button>
          </div>
        </div>
      ) : (
        <>
          {displayText ? (
            <div className="backstory-text">{displayText}</div>
          ) : (
            <p className="empty-state">No backstory text yet. Click "Edit Text" to add one.</p>
          )}
        </>
      )}

      {backstory && (
        <div className="backstory-analysis">
          {backstory.motivations.length > 0 && (
            <div className="analysis-section">
              <h4>Motivations</h4>
              <ul>{backstory.motivations.map((m, i) => <li key={i}>{m}</li>)}</ul>
            </div>
          )}
          {backstory.key_relationships.length > 0 && (
            <div className="analysis-section">
              <h4>Key Relationships</h4>
              <ul>{backstory.key_relationships.map((r, i) => <li key={i}>{r}</li>)}</ul>
            </div>
          )}
          {backstory.secrets.length > 0 && (
            <div className="analysis-section">
              <h4>Secrets</h4>
              <ul>{backstory.secrets.map((s, i) => <li key={i}>{s}</li>)}</ul>
            </div>
          )}
          {backstory.hooks.length > 0 && (
            <div className="analysis-section">
              <h4>Plot Hooks</h4>
              {backstory.hooks.map((h, i) => (
                <div key={i} className="plot-hook">
                  <div className="plot-hook-header">
                    <span className={`hook-priority hook-priority-${h.priority}`}>{h.priority}</span>
                    <span>{h.summary}</span>
                  </div>
                  {h.related_npcs.length > 0 && (
                    <div className="hook-npcs">NPCs: {h.related_npcs.join(', ')}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
