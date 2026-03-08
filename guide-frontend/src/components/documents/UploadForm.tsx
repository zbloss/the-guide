import { useState, useRef } from 'react';

interface UploadFormProps {
  onUpload: (file: File) => Promise<void>;
}

export function UploadForm({ onUpload }: UploadFormProps) {
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!file) { setError('Select a file first'); return; }
    setUploading(true);
    setError('');
    try {
      await onUpload(file);
      setFile(null);
      if (inputRef.current) inputRef.current.value = '';
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="upload-form">
      {error && <div className="form-error-banner">{error}</div>}
      <div className="form-row">
        <input
          ref={inputRef}
          type="file"
          accept=".pdf"
          className="form-input"
          onChange={(e) => setFile(e.target.files?.[0] ?? null)}
        />
        <button type="submit" className="btn btn-primary" disabled={uploading || !file}>
          {uploading ? 'Uploading…' : 'Upload PDF'}
        </button>
      </div>
    </form>
  );
}
