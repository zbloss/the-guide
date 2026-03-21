import { useState, useRef, useEffect } from 'react';
import type { SessionSummary } from '../../api/types';

interface SummaryViewProps {
  summary: SessionSummary;
}

export function SummaryView({ summary }: SummaryViewProps) {
  const [copied, setCopied] = useState(false);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => { if (copyTimerRef.current) clearTimeout(copyTimerRef.current); }, []);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(summary.content);
      setCopied(true);
      copyTimerRef.current = setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard write failed — don't set copied
    }
  };

  const handleDownload = () => {
    const header = `# Session Summary — DM Perspective\n_Generated: ${new Date(summary.generated_at).toLocaleString()}_\n\n`;
    const blob = new Blob([header + summary.content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `session-summary-${summary.session_id}-dm.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="summary-view">
      <div className="summary-meta">
        <span className="badge badge-info">DM Summary</span>
        <span className="summary-date">{new Date(summary.generated_at).toLocaleString()}</span>
        <div className="summary-export-btns">
          <button className="btn btn-sm" onClick={handleCopy}>
            {copied ? '✓ Copied' : 'Copy'}
          </button>
          <button className="btn btn-sm" onClick={handleDownload}>Download .md</button>
        </div>
      </div>
      <div className="summary-content">{summary.content}</div>
    </div>
  );
}
