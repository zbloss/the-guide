import { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { createCampaign } from '../api/campaigns';
import { listGlobalDocs, uploadCampaignDoc, ingestCampaignDoc } from '../api/documents';
import { associateGlobalDoc, getStoryExtractionStatus } from '../api/story';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { Badge } from '../components/common/Badge';
import type { Campaign, GlobalDocument, CampaignDocument, GameSystem, DocumentKind } from '../api/types';

// ── Step indicator ────────────────────────────────────────────────────────────

const STEPS = [
  'Campaign Basics',
  'Select Global Docs',
  'Upload Campaign Doc',
  'Add Characters',
  'Done',
];

function StepIndicator({ current }: { current: number }) {
  return (
    <div style={{ display: 'flex', gap: 0, marginBottom: '2rem', alignItems: 'center' }}>
      {STEPS.map((label, i) => (
        <div key={i} style={{ display: 'flex', alignItems: 'center', flex: 1 }}>
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              flex: 1,
            }}
          >
            <div
              style={{
                width: 28,
                height: 28,
                borderRadius: '50%',
                background:
                  i < current
                    ? 'var(--color-success, #40a02b)'
                    : i === current
                    ? 'var(--color-primary, #89b4fa)'
                    : 'var(--color-surface-2, #2a2a3e)',
                border: '2px solid',
                borderColor:
                  i <= current
                    ? 'var(--color-primary, #89b4fa)'
                    : 'var(--color-border, #555)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: 13,
                fontWeight: 700,
                color: i <= current ? '#fff' : 'var(--color-text-muted, #888)',
              }}
            >
              {i < current ? '✓' : i + 1}
            </div>
            <span
              style={{
                fontSize: 11,
                marginTop: 4,
                textAlign: 'center',
                color:
                  i === current
                    ? 'var(--color-text, #cdd6f4)'
                    : 'var(--color-text-muted, #888)',
                fontWeight: i === current ? 600 : 400,
                whiteSpace: 'nowrap',
              }}
            >
              {label}
            </span>
          </div>
          {i < STEPS.length - 1 && (
            <div
              style={{
                height: 2,
                flex: 1,
                background:
                  i < current
                    ? 'var(--color-primary, #89b4fa)'
                    : 'var(--color-border, #555)',
                margin: '0 4px',
                marginBottom: 22,
              }}
            />
          )}
        </div>
      ))}
    </div>
  );
}

// ── Step 1: Campaign Basics ───────────────────────────────────────────────────

interface Step1Props {
  onCreated: (campaign: Campaign) => void;
}

function Step1Basics({ onCreated }: Step1Props) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [gameSystem, setGameSystem] = useState<GameSystem>('dnd5e');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleNext = async () => {
    if (!name.trim()) {
      setError('Campaign name is required.');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const campaign = await createCampaign({
        name: name.trim(),
        description: description.trim() || undefined,
        game_system: gameSystem,
      });
      onCreated(campaign);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <h2 style={{ marginBottom: '1.25rem' }}>Step 1: Campaign Basics</h2>
      {error && <ErrorBanner message={error} />}
      <div className="form-field" style={{ marginBottom: '1rem' }}>
        <label className="form-label">
          Campaign Name <span style={{ color: 'var(--color-danger, #f38ba8)' }}>*</span>
        </label>
        <input
          className="form-input"
          type="text"
          placeholder="e.g. Lost Mine of Phandelver"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') handleNext(); }}
          autoFocus
        />
      </div>
      <div className="form-field" style={{ marginBottom: '1rem' }}>
        <label className="form-label">Description (optional)</label>
        <textarea
          className="form-input"
          rows={3}
          placeholder="A brief summary of the campaign..."
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </div>
      <div className="form-field" style={{ marginBottom: '1.5rem' }}>
        <label className="form-label">Game System</label>
        <select
          className="form-input"
          value={gameSystem}
          onChange={(e) => setGameSystem(e.target.value as GameSystem)}
        >
          <option value="dnd5e">D&amp;D 5e</option>
          <option value="pathfinder2e">Pathfinder 2e</option>
          <option value="custom">Custom</option>
        </select>
      </div>
      <div className="form-actions">
        <button className="btn btn-primary" onClick={handleNext} disabled={loading}>
          {loading ? <><LoadingSpinner size={14} /> Creating…</> : 'Next →'}
        </button>
      </div>
    </div>
  );
}

// ── Step 2: Select Global Docs ────────────────────────────────────────────────

const KIND_LABELS: Record<string, string> = {
  dm_guide: "Dungeon Master's Guide",
  monster_manual: 'Monster Manual',
  srd: 'SRD',
  campaign: 'Campaign Document',
  supplemental: 'Supplemental',
};

interface Step2Props {
  campaign: Campaign;
  onNext: () => void;
}

function Step2GlobalDocs({ campaign, onNext }: Step2Props) {
  const [docs, setDocs] = useState<GlobalDocument[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [associating, setAssociating] = useState(false);

  useEffect(() => {
    listGlobalDocs()
      .then(setDocs)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  const handleToggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleAssociate = async () => {
    setAssociating(true);
    setError('');
    try {
      await Promise.all(
        Array.from(selected).map((id) => associateGlobalDoc(campaign.id, id))
      );
      onNext();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setAssociating(false);
    }
  };

  // Group docs by kind
  const grouped: Record<string, GlobalDocument[]> = {};
  if (docs) {
    for (const doc of docs) {
      const kind = doc.document_kind ?? 'other';
      if (!grouped[kind]) grouped[kind] = [];
      grouped[kind].push(doc);
    }
  }

  return (
    <div>
      <h2 style={{ marginBottom: '0.5rem' }}>Step 2: Select Global Documents</h2>
      <p style={{ color: 'var(--color-text-muted, #a6adc8)', marginBottom: '1.25rem', fontSize: 14 }}>
        Associate rulebooks and reference documents with this campaign. Skip if none are available.
      </p>

      {error && <ErrorBanner message={error} />}
      {loading && <LoadingSpinner />}

      {docs && docs.length === 0 && (
        <p className="empty-state">No global documents uploaded yet. You can add them later via the Global Documents page.</p>
      )}

      {Object.entries(grouped).map(([kind, kindDocs]) => (
        <div key={kind} style={{ marginBottom: '1rem' }}>
          <h4 style={{ fontSize: 13, color: 'var(--color-text-muted, #a6adc8)', marginBottom: 6, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            {KIND_LABELS[kind] ?? kind}
          </h4>
          {kindDocs.map((doc) => (
            <label
              key={doc.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '0.5rem 0.75rem',
                marginBottom: 4,
                borderRadius: '0.375rem',
                background: selected.has(doc.id)
                  ? 'var(--color-primary-subtle, #1e3a5f)'
                  : 'var(--color-surface-2, #2a2a3e)',
                cursor: 'pointer',
                border: '1px solid',
                borderColor: selected.has(doc.id)
                  ? 'var(--color-primary, #89b4fa)'
                  : 'transparent',
              }}
            >
              <input
                type="checkbox"
                checked={selected.has(doc.id)}
                onChange={() => handleToggle(doc.id)}
                style={{ cursor: 'pointer' }}
              />
              <span style={{ fontSize: 14 }}>{doc.filename}</span>
              <Badge label={doc.ingestion_status} variant={doc.ingestion_status === 'completed' ? 'success' : 'default'} />
            </label>
          ))}
        </div>
      ))}

      <div className="form-actions" style={{ marginTop: '1.5rem', gap: 8 }}>
        <button
          className="btn btn-primary"
          onClick={handleAssociate}
          disabled={associating}
        >
          {associating ? <><LoadingSpinner size={14} /> Associating…</> : selected.size > 0 ? `Associate ${selected.size} doc(s) & Next →` : 'Skip →'}
        </button>
      </div>
    </div>
  );
}

// ── Step 3: Upload Campaign Document ─────────────────────────────────────────

type ExtractionStatus = 'idle' | 'uploading' | 'ingesting' | 'extracting' | 'completed' | 'failed';

interface Step3Props {
  campaign: Campaign;
  onNext: () => void;
}

function Step3Upload({ campaign, onNext }: Step3Props) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [kind, setKind] = useState<DocumentKind>('campaign');
  const [description, setDescription] = useState('');
  const [status, setStatus] = useState<ExtractionStatus>('idle');
  const [error, setError] = useState('');
  const [statusMessage, setStatusMessage] = useState('');
  const pollRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearPoll = () => {
    if (pollRef.current) {
      clearTimeout(pollRef.current);
      pollRef.current = null;
    }
  };

  const pollExtractionStatus = (campaignId: string, docId: string) => {
    clearPoll();
    const check = async () => {
      try {
        const result = await getStoryExtractionStatus(campaignId, docId);
        if (result.status === 'completed') {
          setStatus('completed');
          setStatusMessage('Story extraction complete!');
        } else if (result.status === 'failed') {
          setStatus('failed');
          setStatusMessage(result.error ?? 'Story extraction failed.');
        } else {
          setStatusMessage(`Extracting story… (${result.status})`);
          pollRef.current = setTimeout(check, 3000);
        }
      } catch {
        pollRef.current = setTimeout(check, 3000);
      }
    };
    pollRef.current = setTimeout(check, 3000);
  };

  const handleUpload = async () => {
    if (!file) return;
    setError('');
    setStatus('uploading');
    setStatusMessage('Uploading document…');

    let doc: CampaignDocument;
    try {
      const fd = new FormData();
      fd.append('file', file);
      if (kind) fd.append('kind', kind);
      if (description.trim()) fd.append('description', description.trim());
      doc = await uploadCampaignDoc(campaign.id, fd);
    } catch (e: unknown) {
      setStatus('failed');
      setError(e instanceof Error ? e.message : String(e));
      return;
    }

    setStatus('ingesting');
    setStatusMessage('Ingesting document…');
    try {
      await ingestCampaignDoc(campaign.id, doc.id);
    } catch (e: unknown) {
      setStatus('failed');
      setError(e instanceof Error ? e.message : String(e));
      return;
    }

    setStatus('extracting');
    setStatusMessage('Waiting for story extraction…');
    pollExtractionStatus(campaign.id, doc.id);
  };

  const statusVariant = () => {
    if (status === 'completed') return 'success' as const;
    if (status === 'failed') return 'danger' as const;
    return 'info' as const;
  };

  return (
    <div>
      <h2 style={{ marginBottom: '0.5rem' }}>Step 3: Upload Campaign Document</h2>
      <p style={{ color: 'var(--color-text-muted, #a6adc8)', marginBottom: '1.25rem', fontSize: 14 }}>
        Upload your campaign PDF. The system will extract story arcs, events, and encounters automatically.
      </p>

      {error && <ErrorBanner message={error} />}

      {status === 'idle' || status === 'failed' ? (
        <>
          <div className="form-field" style={{ marginBottom: '1rem' }}>
            <label className="form-label">PDF File</label>
            <input
              ref={fileInputRef}
              type="file"
              accept=".pdf"
              style={{ display: 'none' }}
              onChange={(e) => setFile(e.target.files?.[0] ?? null)}
            />
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <button
                className="btn btn-sm"
                onClick={() => fileInputRef.current?.click()}
                type="button"
              >
                Choose File
              </button>
              <span style={{ fontSize: 13, color: 'var(--color-text-muted, #a6adc8)' }}>
                {file ? file.name : 'No file selected'}
              </span>
            </div>
          </div>
          <div className="form-field" style={{ marginBottom: '1rem' }}>
            <label className="form-label">Document Kind</label>
            <select
              className="form-input"
              value={kind}
              onChange={(e) => setKind(e.target.value as DocumentKind)}
              style={{ width: 'auto' }}
            >
              <option value="campaign">Campaign</option>
              <option value="supplemental">Supplemental</option>
            </select>
          </div>
          {kind === 'supplemental' && (
            <div className="form-field" style={{ marginBottom: '1rem' }}>
              <label className="form-label">Description (optional)</label>
              <input
                className="form-input"
                type="text"
                placeholder="Brief description of the supplemental document"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
            </div>
          )}
          <div className="form-actions" style={{ gap: 8 }}>
            <button
              className="btn btn-primary"
              onClick={handleUpload}
              disabled={!file}
            >
              Upload &amp; Ingest
            </button>
            <button className="btn" onClick={onNext}>
              Skip for now →
            </button>
          </div>
        </>
      ) : (
        <div style={{ marginTop: '1rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: '1rem' }}>
            {status !== 'completed' && <LoadingSpinner />}
            <Badge label={status} variant={statusVariant()} />
            <span style={{ fontSize: 14, color: 'var(--color-text-muted, #a6adc8)' }}>
              {statusMessage}
            </span>
          </div>
          {status === 'completed' && (
            <div className="form-actions" style={{ gap: 8 }}>
              <button
                className="btn"
                onClick={() => {
                  setStatus('idle');
                  setFile(null);
                  setStatusMessage('');
                  setError('');
                  clearPoll();
                  if (fileInputRef.current) fileInputRef.current.value = '';
                }}
              >
                Upload Another →
              </button>
              <button className="btn btn-primary" onClick={onNext}>
                Next →
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Step 4: Add Characters ────────────────────────────────────────────────────

interface Step4Props {
  campaign: Campaign;
  onNext: () => void;
}

function Step4Characters({ campaign, onNext }: Step4Props) {
  return (
    <div>
      <h2 style={{ marginBottom: '0.5rem' }}>Step 4: Add Characters</h2>
      <p style={{ color: 'var(--color-text-muted, #a6adc8)', marginBottom: '0.75rem', fontSize: 14 }}>
        You can add player characters and NPCs from the <strong>Characters</strong> tab on your campaign page.
      </p>
      <p style={{ marginBottom: '1.5rem', fontSize: 14 }}>
        <a
          href={`/campaigns/${campaign.id}/characters`}
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: 'var(--color-primary, #89b4fa)' }}
        >
          Open Characters tab in new tab ↗
        </a>
      </p>
      <div className="form-actions">
        <button className="btn btn-primary" onClick={onNext}>
          Continue →
        </button>
      </div>
    </div>
  );
}

// ── Step 5: Done ──────────────────────────────────────────────────────────────

interface Step5Props {
  campaign: Campaign;
}

function Step5Done({ campaign }: Step5Props) {
  const navigate = useNavigate();

  return (
    <div style={{ textAlign: 'center', padding: '2rem 0' }}>
      <div style={{ fontSize: 48, marginBottom: '1rem' }}>🎲</div>
      <h2 style={{ marginBottom: '0.5rem' }}>Campaign Setup Complete!</h2>
      <p style={{ color: 'var(--color-text-muted, #a6adc8)', marginBottom: '2rem', fontSize: 15 }}>
        <strong>{campaign.name}</strong> is ready. Jump into your story or explore the campaign.
      </p>
      <div className="form-actions" style={{ justifyContent: 'center', gap: 12 }}>
        <button
          className="btn btn-primary"
          onClick={() => navigate(`/campaigns/${campaign.id}/story`)}
        >
          Go to Story →
        </button>
        <button
          className="btn"
          onClick={() => navigate(`/campaigns/${campaign.id}/characters`)}
        >
          Go to Campaign →
        </button>
      </div>
    </div>
  );
}

// ── Wizard Shell ──────────────────────────────────────────────────────────────

export function CampaignWizardPage() {
  const [step, setStep] = useState(0);
  const [campaign, setCampaign] = useState<Campaign | null>(null);

  return (
    <div className="page">
      <div className="page-header">
        <h1>New Campaign</h1>
      </div>

      <div
        style={{
          maxWidth: 640,
          margin: '0 auto',
          padding: '1.5rem',
          background: 'var(--color-surface, #1e1e2e)',
          borderRadius: '0.75rem',
          border: '1px solid var(--color-border, #333)',
        }}
      >
        <StepIndicator current={step} />

        {step === 0 && (
          <Step1Basics
            onCreated={(c) => {
              setCampaign(c);
              setStep(1);
            }}
          />
        )}
        {step === 1 && campaign && (
          <Step2GlobalDocs campaign={campaign} onNext={() => setStep(2)} />
        )}
        {step === 2 && campaign && (
          <Step3Upload campaign={campaign} onNext={() => setStep(3)} />
        )}
        {step === 3 && campaign && (
          <Step4Characters campaign={campaign} onNext={() => setStep(4)} />
        )}
        {step === 4 && campaign && <Step5Done campaign={campaign} />}
      </div>
    </div>
  );
}
