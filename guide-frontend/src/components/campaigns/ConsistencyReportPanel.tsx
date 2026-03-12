import { useState } from 'react';
import { runConsistencyCheck } from '../../api/campaigns';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { Badge } from '../common/Badge';
import type { ConsistencyReport } from '../../api/types';

interface ConsistencyReportPanelProps {
  campaignId: string;
}

export function ConsistencyReportPanel({ campaignId }: ConsistencyReportPanelProps) {
  const [report, setReport] = useState<ConsistencyReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [expanded, setExpanded] = useState(false);

  const handleCheck = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await runConsistencyCheck(campaignId);
      setReport(result);
      setExpanded(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="consistency-panel">
      <div className="section-header">
        <h3>Lore Consistency</h3>
        <button
          className="btn btn-sm btn-primary"
          onClick={handleCheck}
          disabled={loading}
        >
          {loading ? <><LoadingSpinner size={14} /> Checking…</> : '🔍 Check Consistency'}
        </button>
      </div>

      {error && <div className="form-error-banner">{error}</div>}

      {report && (
        <div className="consistency-report">
          <div
            className="consistency-summary"
            onClick={() => setExpanded((e) => !e)}
            style={{ cursor: 'pointer' }}
          >
            <span>{expanded ? '▼' : '▶'}</span>
            <strong>{report.summary}</strong>
            <Badge
              label={`${report.issues.length} issue${report.issues.length !== 1 ? 's' : ''}`}
              variant={report.issues.length === 0 ? 'default' : report.issues.some(i => i.severity === 'major') ? 'danger' : 'warning'}
            />
          </div>

          {expanded && report.issues.length > 0 && (
            <ul className="consistency-issues">
              {report.issues.map((issue, idx) => (
                <li key={idx} className={`consistency-issue issue-${issue.severity}`}>
                  <Badge label={issue.category} variant="info" />
                  <Badge label={issue.severity} variant={issue.severity === 'major' ? 'danger' : 'warning'} />
                  <span>{issue.description}</span>
                </li>
              ))}
            </ul>
          )}

          {expanded && report.issues.length === 0 && (
            <p className="empty-state">No consistency issues found. Your lore is tight!</p>
          )}
        </div>
      )}
    </div>
  );
}
