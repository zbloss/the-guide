import { useState, useRef, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCharacters, createCharacter, deleteCharacter, importCharactersCsv, importDndBeyond, parseCharacterSheet } from '../api/characters';
import { CharacterList } from '../components/characters/CharacterList';
import { CharacterForm } from '../components/characters/CharacterForm';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { cacheItems } from '../lib/offlineDb';
import type { Character, CreateCharacterRequest, ParsedSheetResult } from '../api/types';

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
  const pdfInputRef = useRef<HTMLInputElement>(null);
  const [ddbImporting, setDdbImporting] = useState(false);
  const [pdfParsing, setPdfParsing] = useState(false);
  const [pdfResult, setPdfResult] = useState<ParsedSheetResult | null>(null);

  useEffect(() => {
    if (characters) {
      cacheItems('characters', characters).catch(console.error);
    }
  }, [characters]);

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

  const handlePdfParse = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setPdfParsing(true);
    setCsvError('');
    try {
      const result = await parseCharacterSheet(campaignId!, file);
      setPdfResult(result);
      setShowCreate(true);
    } catch (err: unknown) {
      setCsvError(err instanceof Error ? err.message : String(err));
    } finally {
      setPdfParsing(false);
      if (pdfInputRef.current) pdfInputRef.current.value = '';
    }
  };

  const handleCreate = async (data: CreateCharacterRequest) => {
    await createCharacter(campaignId!, data);
    setShowCreate(false);
    setPdfResult(null);
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
          <input ref={pdfInputRef} type="file" accept=".pdf" style={{ display: 'none' }} onChange={handlePdfParse} />
          <button className="btn btn-sm" onClick={() => csvInputRef.current?.click()} disabled={csvImporting} title="Import from CSV (name,type,class,race,level,max_hp,ac,speed)">
            {csvImporting ? 'Importing…' : 'Import CSV'}
          </button>
          <button className="btn btn-sm" onClick={() => ddbInputRef.current?.click()} disabled={ddbImporting} title="Import from D&D Beyond JSON export">
            {ddbImporting ? 'Importing…' : 'D&D Beyond'}
          </button>
          <button className="btn btn-sm" onClick={() => pdfInputRef.current?.click()} disabled={pdfParsing} title="Import from PDF character sheet">
            {pdfParsing ? 'Parsing…' : 'Upload PDF Sheet'}
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
        <Modal title="New Character" onClose={() => { setShowCreate(false); setPdfResult(null); }}>
          <CharacterForm
            onSubmit={handleCreate}
            onCancel={() => { setShowCreate(false); setPdfResult(null); }}
            initialValues={pdfResult ? {
              name: pdfResult.name,
              character_type: 'pc',
              class: pdfResult.class ?? undefined,
              race: pdfResult.race ?? undefined,
              level: pdfResult.level,
              max_hp: pdfResult.max_hp,
              armor_class: pdfResult.armor_class,
              speed: pdfResult.speed,
              ability_scores: pdfResult.ability_scores,
              backstory_text: pdfResult.backstory_text ?? undefined,
              parse_confidence: pdfResult.parse_confidence,
            } : undefined}
          />
        </Modal>
      )}
    </div>
  );
}
