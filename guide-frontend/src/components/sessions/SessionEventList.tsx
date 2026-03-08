import { Badge } from '../common/Badge';
import type { SessionEvent } from '../../api/types';

interface SessionEventListProps {
  events: SessionEvent[];
}

const SIG_VARIANT: Record<string, 'default' | 'info' | 'warning' | 'danger'> = {
  minor: 'default',
  moderate: 'info',
  major: 'warning',
  critical: 'danger',
};

export function SessionEventList({ events }: SessionEventListProps) {
  if (events.length === 0) {
    return <p className="empty-state">No events recorded yet.</p>;
  }
  return (
    <table className="data-table">
      <thead>
        <tr>
          <th>Type</th>
          <th>Description</th>
          <th>Significance</th>
          <th>Visible</th>
          <th>Time</th>
        </tr>
      </thead>
      <tbody>
        {events.map((ev) => (
          <tr key={ev.id}>
            <td><Badge label={ev.event_type} variant="info" /></td>
            <td>{ev.description}</td>
            <td><Badge label={ev.significance} variant={SIG_VARIANT[ev.significance] ?? 'default'} /></td>
            <td>{ev.is_player_visible ? '✓' : '—'}</td>
            <td>{new Date(ev.created_at).toLocaleTimeString()}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
