// Web-based save dialog for dev environments (Tauri dev popup, browser dev)
// Uses browser download API with filename editing - shows modal before auto-download
import { useState, useEffect } from 'react';

interface Props {
  open: boolean;
  defaultFileName: string;
  content: string;
  onClose: () => void;
  onSaved?: (info: string) => void;
}

const ensureTxt = (name: string) => (name.endsWith('.txt') ? name : `${name}.txt`);

export function SaveTranscriptWebDialog({ open, defaultFileName, content, onClose, onSaved }: Props) {
  const [fileName, setFileName] = useState<string>(ensureTxt(defaultFileName));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Update fileName when defaultFileName prop changes
  useEffect(() => {
    if (open) {
      setFileName(ensureTxt(defaultFileName));
      setError(null);
    }
  }, [open, defaultFileName]);

  if (!open) return null;

  const browserDownload = (name: string) => {
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = ensureTxt(name);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    setTimeout(() => URL.revokeObjectURL(link.href), 100);
    onSaved?.(link.download);
  };

  const save = async () => {
    setError(null);
    const name = ensureTxt(fileName.trim() || defaultFileName);
    setSaving(true);
    try {
      const anyWindow = window as any;
      if (anyWindow.showSaveFilePicker) {
        const handle = await anyWindow.showSaveFilePicker({
          suggestedName: name,
          types: [
            {
              description: 'Text',
              accept: { 'text/plain': ['.txt'] }
            }
          ]
        });
        const writable = await handle.createWritable();
        await writable.write(content);
        await writable.close();
        onSaved?.(name);
      } else {
        // Fallback to standard download
        browserDownload(name);
      }
      onClose();
    } catch (e: any) {
      if (e?.name === 'AbortError') {
        // user cancelled dialog
        onClose();
      } else {
        setError(e?.message ?? 'Failed to save');
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000 }}>
      <div style={{ background: '#fff', width: 520, maxWidth: '90vw', borderRadius: 12, boxShadow: '0 8px 32px rgba(0,0,0,0.2)' }}>
        <div style={{ padding: '16px 20px', borderBottom: '1px solid #eee', fontWeight: 600 }}>Save Transcript</div>
        <div style={{ padding: 20 }}>
          <div style={{ marginBottom: 12 }}>
            <label style={{ display: 'block', fontSize: 12, color: '#555', marginBottom: 6 }}>File name</label>
            <input
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              placeholder="chat-YYYY-MM-DD.txt"
              style={{ width: '100%', padding: '10px 12px', border: '1px solid #ddd', borderRadius: 8 }}
            />
          </div>
          <div style={{ color: '#666', fontSize: 12, marginTop: 4 }}>
            Your file will save to the default Downloads folder.
          </div>
          {error && <div style={{ color: '#b00020', fontSize: 13, marginTop: 8 }}>{error}</div>}
        </div>
        <div style={{ padding: 16, display: 'flex', justifyContent: 'flex-end', gap: 8, borderTop: '1px solid #eee' }}>
          <button onClick={onClose} disabled={saving} style={{ padding: '10px 14px', borderRadius: 8, border: '1px solid #ddd', background: '#f5f5f5' }}>Cancel</button>
          <button onClick={save} disabled={saving} style={{ padding: '10px 14px', borderRadius: 8, border: '1px solid #0b5', background: '#0c6', color: '#fff' }}>{saving ? 'Saving…' : 'Save'}</button>
        </div>
      </div>
    </div>
  );
}