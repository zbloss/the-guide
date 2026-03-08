import type { SessionSummary } from '../../api/types';

interface SummaryViewProps {
  summary: SessionSummary;
}

export function SummaryView({ summary }: SummaryViewProps) {
  return (
    <div className="summary-view">
      <div className="summary-meta">
        <span className="badge badge-info">{summary.perspective === 'dm' ? 'DM Summary' : 'Player Summary'}</span>
        <span className="summary-date">{new Date(summary.generated_at).toLocaleString()}</span>
      </div>
      <div className="summary-content">{summary.content}</div>
    </div>
  );
}
