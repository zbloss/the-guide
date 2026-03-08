import { useState } from 'react';
import { analyzeBackstory } from '../../api/characters';
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

  const handleAnalyze = async () => {
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

  return (
    <div className="backstory-panel">
      <div className="section-header">
        <h3>Backstory</h3>
        <button className="btn btn-sm btn-primary" onClick={handleAnalyze} disabled={analyzing}>
          {analyzing ? <><LoadingSpinner size={14} /> Analyzing…</> : 'Analyze with AI'}
        </button>
      </div>

      {error && <div className="form-error-banner">{error}</div>}

      {backstory?.raw_text && (
        <div className="backstory-text">{backstory.raw_text}</div>
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

      {!backstory && !analyzing && (
        <p className="empty-state">No backstory analysis yet. Click "Analyze with AI" to extract plot hooks and motivations.</p>
      )}
    </div>
  );
}
