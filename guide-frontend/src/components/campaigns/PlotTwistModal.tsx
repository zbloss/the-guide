import { useState } from 'react';
import { generatePlotTwist } from '../../api/campaigns';
import { Modal } from '../common/Modal';
import { LoadingSpinner } from '../common/LoadingSpinner';

interface PlotTwistModalProps {
  campaignId: string;
}

type Tone = 'dramatic' | 'comedic' | 'tragic';

export function PlotTwistModal({ campaignId }: PlotTwistModalProps) {
  const [open, setOpen] = useState(false);
  const [tone, setTone] = useState<Tone>('dramatic');
  const [result, setResult] = useState<{ twist: string; tone: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);

  const handleGenerate = async () => {
    setLoading(true);
    setError('');
    setResult(null);
    setCopied(false);
    try {
      const data = await generatePlotTwist(campaignId, tone);
      setResult(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = () => {
    if (result) {
      navigator.clipboard.writeText(result.twist).then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      });
    }
  };

  const handleClose = () => {
    setOpen(false);
    setResult(null);
    setError('');
    setCopied(false);
  };

  return (
    <>
      <button className="btn btn-sm btn-primary" onClick={() => setOpen(true)}>
        Plot Twist
      </button>

      {open && (
        <Modal title="Generate Plot Twist" onClose={handleClose}>
          <div className="plot-twist-modal">
            <div className="form-field">
              <label className="form-label">Tone</label>
              <select
                className="form-input"
                value={tone}
                onChange={(e) => setTone(e.target.value as Tone)}
                disabled={loading}
              >
                <option value="dramatic">Dramatic</option>
                <option value="comedic">Comedic</option>
                <option value="tragic">Tragic</option>
              </select>
            </div>

            <div className="form-actions">
              <button
                className="btn btn-primary"
                onClick={handleGenerate}
                disabled={loading}
              >
                {loading ? <><LoadingSpinner size={14} /> Generating...</> : 'Generate'}
              </button>
              <button className="btn" onClick={handleClose} disabled={loading}>
                Cancel
              </button>
            </div>

            {error && <div className="form-error-banner">{error}</div>}

            {result && (
              <div className="plot-twist-result">
                <div className="plot-twist-card">
                  <div className="plot-twist-header">
                    <strong>Plot Twist</strong>
                    <span className="badge badge-info">{result.tone}</span>
                  </div>
                  <p className="plot-twist-text">{result.twist}</p>
                </div>
                <div className="form-actions" style={{ marginTop: '0.5rem' }}>
                  <button className="btn btn-sm" onClick={handleCopy}>
                    {copied ? 'Copied!' : 'Copy to Clipboard'}
                  </button>
                </div>
              </div>
            )}
          </div>
        </Modal>
      )}
    </>
  );
}
