import { useState } from 'react';
import { useApi } from '../../hooks/useApi';
import { listTemplates, deleteTemplate } from '../../api/templates';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { ErrorBanner } from '../common/ErrorBanner';
import type { EncounterTemplate } from '../../api/types';

interface TemplateLibraryProps {
  onLoad: (template: EncounterTemplate) => void;
}

export function TemplateLibrary({ onLoad }: TemplateLibraryProps) {
  const { data: templates, loading, error, refetch } = useApi<EncounterTemplate[]>(
    () => listTemplates(),
    [],
  );
  const [deleteError, setDeleteError] = useState('');

  const handleDelete = async (id: string) => {
    setDeleteError('');
    try {
      await deleteTemplate(id);
      refetch();
    } catch (e: unknown) {
      setDeleteError(e instanceof Error ? e.message : String(e));
    }
  };

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBanner message={error} />;

  return (
    <div className="template-library">
      <h4>Saved Templates</h4>
      {deleteError && <ErrorBanner message={deleteError} />}
      {(!templates || templates.length === 0) ? (
        <p className="empty-state">No templates saved yet. Use "Save as Template" on an encounter to create one.</p>
      ) : (
        <ul className="template-list">
          {templates.map((tmpl) => (
            <li key={tmpl.id} className="template-item card">
              <div className="template-info">
                <strong>{tmpl.name}</strong>
                {tmpl.description && <p className="template-description">{tmpl.description}</p>}
                <span className="template-meta">
                  {tmpl.participants.length} participant{tmpl.participants.length !== 1 ? 's' : ''}
                  {tmpl.participants.length > 0 && (
                    <> &mdash; {tmpl.participants.map((p) => p.name).join(', ')}</>
                  )}
                </span>
              </div>
              <div className="template-actions btn-group">
                <button
                  className="btn btn-sm btn-primary"
                  onClick={() => onLoad(tmpl)}
                >
                  Load
                </button>
                <button
                  className="btn btn-sm btn-danger"
                  onClick={() => handleDelete(tmpl.id)}
                >
                  Delete
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
