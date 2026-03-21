import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCampaignDocs, uploadCampaignDoc, getCampaignDoc, ingestCampaignDoc } from '../api/documents';
import { DocumentList } from '../components/documents/DocumentList';
import { UploadForm } from '../components/documents/UploadForm';
import { IngestButton } from '../components/documents/IngestButton';
import { OcrPreviewModal } from '../components/documents/OcrPreviewModal';
import { DocumentChunkSearch } from '../components/documents/DocumentChunkSearch';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { CampaignDocument } from '../api/types';

export function DocumentsPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: docs, loading, error, refetch } = useApi<CampaignDocument[]>(
    () => listCampaignDocs(campaignId!),
    [campaignId],
  );
  const [previewDocId, setPreviewDocId] = useState<string | null>(null);

  const handleUpload = async (file: File) => {
    await uploadCampaignDoc(campaignId!, file);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Campaign Documents</h2>
      </div>

      <UploadForm onUpload={handleUpload} />

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}

      {docs && (
        <>
          <DocumentList
            documents={docs}
            renderActions={(doc) => {
              const d = doc as CampaignDocument;
              return (
                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                  <IngestButton
                    docId={d.id}
                    currentStatus={d.ingestion_status}
                    onIngest={() => ingestCampaignDoc(campaignId!, d.id).then(() => {})}
                    onPoll={() => getCampaignDoc(campaignId!, d.id)}
                    onComplete={refetch}
                  />
                  {d.ingestion_status === 'completed' && (
                    <button className="btn btn-sm" onClick={() => setPreviewDocId(d.id)}>
                      Preview OCR
                    </button>
                  )}
                </div>
              );
            }}
          />
          <DocumentChunkSearch campaignId={campaignId!} documents={docs} />
        </>
      )}

      {previewDocId && (
        <OcrPreviewModal
          campaignId={campaignId!}
          docId={previewDocId}
          filename={docs?.find((d) => d.id === previewDocId)?.filename ?? ''}
          onClose={() => setPreviewDocId(null)}
        />
      )}
    </div>
  );
}
