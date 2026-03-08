import { StatusBadge } from '../common/Badge';
import type { CampaignDocument, GlobalDocument } from '../../api/types';

type AnyDoc = CampaignDocument | GlobalDocument;

interface DocumentListProps {
  documents: AnyDoc[];
  renderActions?: (doc: AnyDoc) => React.ReactNode;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function DocumentList({ documents, renderActions }: DocumentListProps) {
  if (documents.length === 0) {
    return <p className="empty-state">No documents uploaded yet.</p>;
  }
  return (
    <table className="data-table">
      <thead>
        <tr>
          <th>Filename</th>
          <th>Size</th>
          <th>Status</th>
          <th>Uploaded</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {documents.map((doc) => (
          <tr key={doc.id}>
            <td>{doc.filename}</td>
            <td>{formatBytes(doc.file_size)}</td>
            <td><StatusBadge status={doc.status} /></td>
            <td>{new Date(doc.uploaded_at).toLocaleDateString()}</td>
            <td>{renderActions?.(doc)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
