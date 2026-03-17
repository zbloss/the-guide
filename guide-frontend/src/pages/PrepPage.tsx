import { useState, useEffect, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCharacters } from '../api/characters';
import {
  listPrepResults,
  generateSessionRecap,
  generateStorySoFar,
  generateStoryAhead,
  generateCharacterRoadmap,
} from '../api/prep';
import type { DmPrepResult, Character } from '../api/types';
import { ErrorBanner } from '../components/common/ErrorBanner';

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function PrepSection({
  title,
  result,
  loading,
  error,
  onGenerate,
  onRegenerate,
  extra,
}: {
  title: string;
  result: DmPrepResult | null;
  loading: boolean;
  error: string;
  onGenerate: () => void;
  onRegenerate: () => void;
  extra?: React.ReactNode;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    if (result) {
      navigator.clipboard.writeText(result.content).catch(() => {});
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="prep-section">
      <div className="prep-section-header">
        <h3>{title}</h3>
        {result && (
          <span className="prep-meta">Last generated: {formatDate(result.generated_at)}</span>
        )}
      </div>
      {extra}
      <div className="prep-actions">
        {!result ? (
          <button className="btn btn-primary btn-sm" onClick={onGenerate} disabled={loading}>
            {loading ? 'Generating…' : 'Generate'}
          </button>
        ) : (
          <>
            <button className="btn btn-sm" onClick={onRegenerate} disabled={loading}>
              {loading ? 'Regenerating…' : 'Regenerate'}
            </button>
            <button className="btn btn-sm" onClick={handleCopy}>
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </>
        )}
      </div>
      {error && <ErrorBanner message={error} />}
      {result && (
        <pre className="prep-content">{result.content}</pre>
      )}
    </div>
  );
}

function CharacterRoadmapSection({
  pcs,
  roadmapResults,
  roadmapLoading,
  roadmapErrors,
  currentChapter,
  onGenerate,
}: {
  pcs: Character[];
  roadmapResults: Record<string, DmPrepResult>;
  roadmapLoading: Record<string, boolean>;
  roadmapErrors: Record<string, string>;
  currentChapter: string;
  onGenerate: (charId: string, force: boolean) => void;
}) {
  const [selectedCharId, setSelectedCharId] = useState(pcs[0]?.id ?? '');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (pcs.length > 0 && !selectedCharId) {
      setSelectedCharId(pcs[0].id);
    }
  }, [pcs]);

  // Reset copied state when selection changes
  useEffect(() => {
    setCopied(false);
  }, [selectedCharId]);

  const result = selectedCharId ? (roadmapResults[selectedCharId] ?? null) : null;
  const loading = selectedCharId ? (roadmapLoading[selectedCharId] ?? false) : false;
  const error = selectedCharId ? (roadmapErrors[selectedCharId] ?? '') : '';

  const handleCopy = () => {
    if (result) {
      navigator.clipboard.writeText(result.content).catch(() => {});
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // Suppress unused warning — currentChapter is used by the parent via onGenerate
  void currentChapter;

  if (pcs.length === 0) {
    return (
      <div className="prep-section">
        <div className="prep-section-header"><h3>Character Roadmaps</h3></div>
        <p className="prep-meta">No player characters found in this campaign.</p>
      </div>
    );
  }

  return (
    <div className="prep-section">
      <div className="prep-section-header">
        <h3>Character Roadmaps</h3>
        {result && <span className="prep-meta">Last generated: {formatDate(result.generated_at)}</span>}
      </div>
      <div className="prep-actions">
        <select
          className="form-input"
          style={{ width: 'auto' }}
          value={selectedCharId}
          onChange={(e) => setSelectedCharId(e.target.value)}
        >
          {pcs.map((pc) => (
            <option key={pc.id} value={pc.id}>
              {pc.name} ({pc.class ?? 'Unknown'} Lv{pc.level})
            </option>
          ))}
        </select>
      </div>
      <div className="prep-actions">
        {!result ? (
          <button className="btn btn-primary btn-sm" onClick={() => onGenerate(selectedCharId, false)} disabled={loading}>
            {loading ? 'Generating…' : 'Generate Roadmap'}
          </button>
        ) : (
          <>
            <button className="btn btn-sm" onClick={() => onGenerate(selectedCharId, true)} disabled={loading}>
              {loading ? 'Regenerating…' : 'Regenerate'}
            </button>
            <button className="btn btn-sm" onClick={handleCopy}>
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </>
        )}
      </div>
      {error && <ErrorBanner message={error} />}
      {result && <pre className="prep-content">{result.content}</pre>}
    </div>
  );
}

export function PrepPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: characters } = useApi<Character[]>(() => listCharacters(campaignId!), [campaignId]);

  const pcs = characters?.filter((c) => c.character_type === 'pc') ?? [];

  const [currentChapter, setCurrentChapter] = useState('');

  // Session Recap state
  const [recapResult, setRecapResult] = useState<DmPrepResult | null>(null);
  const [recapLoading, setRecapLoading] = useState(false);
  const [recapError, setRecapError] = useState('');

  // Story So Far state
  const [soFarResult, setSoFarResult] = useState<DmPrepResult | null>(null);
  const [soFarLoading, setSoFarLoading] = useState(false);
  const [soFarError, setSoFarError] = useState('');

  // Story Ahead state
  const [aheadResult, setAheadResult] = useState<DmPrepResult | null>(null);
  const [aheadLoading, setAheadLoading] = useState(false);
  const [aheadError, setAheadError] = useState('');

  // Character Roadmap state (per char id)
  const [roadmapResults, setRoadmapResults] = useState<Record<string, DmPrepResult>>({});
  const [roadmapLoading, setRoadmapLoading] = useState<Record<string, boolean>>({});
  const [roadmapErrors, setRoadmapErrors] = useState<Record<string, string>>({});

  const [hydrateError, setHydrateError] = useState('');

  // Hydrate cached results on mount
  useEffect(() => {
    if (!campaignId) return;
    listPrepResults(campaignId).then((results) => {
      for (const r of results) {
        if (r.prep_type === 'session_recap' && !r.character_id) setRecapResult(r);
        if (r.prep_type === 'story_so_far' && !r.character_id) setSoFarResult(r);
        if (r.prep_type === 'story_ahead' && !r.character_id) setAheadResult(r);
        if (r.prep_type === 'character_roadmap' && r.character_id) {
          setRoadmapResults((prev) => ({ ...prev, [r.character_id!]: r }));
        }
      }
    }).catch((e: unknown) => {
      setHydrateError(e instanceof Error ? e.message : 'Failed to load cached prep results');
    });
  }, [campaignId]);

  const handleGenerateRecap = useCallback(async (force = false) => {
    setRecapLoading(true);
    setRecapError('');
    try {
      const result = await generateSessionRecap(campaignId!, { force_regenerate: force || undefined });
      setRecapResult(result);
    } catch (e: unknown) {
      setRecapError(e instanceof Error ? e.message : String(e));
    } finally {
      setRecapLoading(false);
    }
  }, [campaignId]);

  const handleGenerateSoFar = useCallback(async (force = false) => {
    setSoFarLoading(true);
    setSoFarError('');
    try {
      const result = await generateStorySoFar(campaignId!, {
        current_chapter: currentChapter || undefined,
        force_regenerate: force || undefined,
      });
      setSoFarResult(result);
    } catch (e: unknown) {
      setSoFarError(e instanceof Error ? e.message : String(e));
    } finally {
      setSoFarLoading(false);
    }
  }, [campaignId, currentChapter]);

  const handleGenerateAhead = useCallback(async (force = false) => {
    setAheadLoading(true);
    setAheadError('');
    try {
      const result = await generateStoryAhead(campaignId!, {
        current_chapter: currentChapter || undefined,
        force_regenerate: force || undefined,
      });
      setAheadResult(result);
    } catch (e: unknown) {
      setAheadError(e instanceof Error ? e.message : String(e));
    } finally {
      setAheadLoading(false);
    }
  }, [campaignId, currentChapter]);

  const handleGenerateRoadmap = useCallback(async (charId: string, force = false) => {
    setRoadmapLoading((prev) => ({ ...prev, [charId]: true }));
    setRoadmapErrors((prev) => ({ ...prev, [charId]: '' }));
    try {
      const result = await generateCharacterRoadmap(campaignId!, charId, {
        current_chapter: currentChapter || undefined,
        force_regenerate: force || undefined,
      });
      setRoadmapResults((prev) => ({ ...prev, [charId]: result }));
    } catch (e: unknown) {
      setRoadmapErrors((prev) => ({ ...prev, [charId]: e instanceof Error ? e.message : String(e) }));
    } finally {
      setRoadmapLoading((prev) => ({ ...prev, [charId]: false }));
    }
  }, [campaignId, currentChapter]);

  return (
    <div className="prep-page">
      {hydrateError && <ErrorBanner message={hydrateError} />}
      <div className="prep-header">
        <h2>DM Prep Suite</h2>
        <p className="prep-description">
          Generate structured prep materials grounded in your session history and campaign documents.
        </p>
      </div>

      <div className="prep-chapter-input">
        <label className="form-label">Current Story Position (chapter/location)</label>
        <input
          className="form-input"
          type="text"
          placeholder="e.g. Chapter 3, The Sunless Citadel"
          value={currentChapter}
          onChange={(e) => setCurrentChapter(e.target.value)}
        />
        <span className="prep-meta">Used by Story So Far, Story Ahead, and Character Roadmaps</span>
      </div>

      <PrepSection
        title="Session Recap"
        result={recapResult}
        loading={recapLoading}
        error={recapError}
        onGenerate={() => handleGenerateRecap(false)}
        onRegenerate={() => handleGenerateRecap(true)}
      />

      <PrepSection
        title="Story So Far"
        result={soFarResult}
        loading={soFarLoading}
        error={soFarError}
        onGenerate={() => handleGenerateSoFar(false)}
        onRegenerate={() => handleGenerateSoFar(true)}
      />

      <PrepSection
        title="Story Ahead"
        result={aheadResult}
        loading={aheadLoading}
        error={aheadError}
        onGenerate={() => handleGenerateAhead(false)}
        onRegenerate={() => handleGenerateAhead(true)}
      />

      <CharacterRoadmapSection
        pcs={pcs}
        roadmapResults={roadmapResults}
        roadmapLoading={roadmapLoading}
        roadmapErrors={roadmapErrors}
        currentChapter={currentChapter}
        onGenerate={handleGenerateRoadmap}
      />
    </div>
  );
}
