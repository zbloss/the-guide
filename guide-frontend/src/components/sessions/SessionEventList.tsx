import { Badge } from '../common/Badge';
import { ConfirmButton } from '../common/ConfirmButton';
import type { SessionEvent } from '../../api/types';

interface SessionEventListProps {
  events: SessionEvent[];
  onDelete?: (eventId: string) => void;
}

const SIG_VARIANT: Record<string, 'default' | 'info' | 'warning' | 'danger'> = {
  minor: 'default',
  major: 'warning',
  milestone: 'warning',
};

export function SessionEventList({ events, onDelete }: SessionEventListProps) {
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
          <th>Time</th>
          {onDelete && <th></th>}
        </tr>
      </thead>
      <tbody>
        {events.map((ev) => (
          <tr key={ev.id}>
            <td><Badge label={ev.event_type} variant="info" /></td>
            <td>{ev.description}</td>
            <td><Badge label={ev.significance} variant={SIG_VARIANT[ev.significance] ?? 'default'} /></td>
            <td>{new Date(ev.occurred_at).toLocaleTimeString(undefined, { timeZoneName: 'short' })}</td>
            {onDelete && (
              <td>
                <ConfirmButton label="Delete" variant="danger" onConfirm={() => onDelete(ev.id)} />
              </td>
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
