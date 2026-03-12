import { useState, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCharacters, createCharacter, deleteCharacter, importCharactersCsv, importDndBeyond } from '../api/characters';
import { CharacterList } from '../components/characters/CharacterList';
import { CharacterForm } from '../components/characters/CharacterForm';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Character, CreateCharacterRequest } from '../api/types';

export function CharactersPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: characters, loading, error, refetch } = useApi<Character[]>(
    () => listCharacters(campaignId!),
    [campaignId],
  );
  const [showCreate, setShowCreate] = useState(false);
  const [csvImporting, setCsvImporting] = useState(false);
  const [csvError, setCsvError] = useState('');
  const [csvResult, setCsvResult] = useState('');
  const csvInputRef = useRef<HTMLInputElement>(null);
  const ddbInputRef = useRef<HTMLInputElement>(null);
  const [ddbImporting, setDdbImporting] = useState(false);

  const handleCsvImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setCsvImporting(true);
    setCsvError('');
    setCsvResult('');
    try {
      const result = await importCharactersCsv(campaignId!, file);
      setCsvResult(`Imported ${result.imported} character${result.imported !== 1 ? 's' : ''}.`);
      refetch();
    } catch (err: unknown) {
      setCsvError(err instanceof Error ? err.message : String(err));
    } finally {
      setCsvImporting(false);
      if (csvInputRef.current) csvInputRef.current.value = '';
    }
  };

  const handleDdbImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setDdbImporting(true);
    setCsvError('');
    setCsvResult('');
    try {
      const text = await file.text();
      const json = JSON.parse(text);
      await importDndBeyond(campaignId!, json);
      setCsvResult('D&D Beyond character imported.');
      refetch();
    } catch (err: unknown) {
      setCsvError(err instanceof Error ? err.message : String(err));
    } finally {
      setDdbImporting(false);
      if (ddbInputRef.current) ddbInputRef.current.value = '';
    }
  };

  const handleCreate = async (data: CreateCharacterRequest) => {
    await createCharacter(campaignId!, data);
    setShowCreate(false);
    refetch();
  };

  const handleDelete = async (charId: string) => {
    await deleteCharacter(campaignId!, charId);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Characters</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <input ref={csvInputRef} type="file" accept=".csv" style={{ display: 'none' }} onChange={handleCsvImport} />
          <input ref={ddbInputRef} type="file" accept=".json" style={{ display: 'none' }} onChange={handleDdbImport} />
          <button className="btn btn-sm" onClick={() => csvInputRef.current?.click()} disabled={csvImporting} title="Import from CSV (name,type,class,race,level,max_hp,ac,speed)">
            {csvImporting ? 'Importing…' : 'Import CSV'}
          </button>
          <button className="btn btn-sm" onClick={() => ddbInputRef.current?.click()} disabled={ddbImporting} title="Import from D&D Beyond JSON export">
            {ddbImporting ? 'Importing…' : 'D&D Beyond'}
          </button>
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>+ Add Character</button>
        </div>
      </div>
      {csvError && <div className="form-error-banner" style={{ marginBottom: 8 }}>{csvError}</div>}
      {csvResult && <div className="form-success-banner" style={{ marginBottom: 8 }}>{csvResult}</div>}

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {characters && <CharacterList characters={characters} campaignId={campaignId!} onDelete={handleDelete} />}

      {showCreate && (
        <Modal title="New Character" onClose={() => setShowCreate(false)}>
          <CharacterForm onSubmit={handleCreate} onCancel={() => setShowCreate(false)} />
        </Modal>
      )}
    </div>
  );
}
