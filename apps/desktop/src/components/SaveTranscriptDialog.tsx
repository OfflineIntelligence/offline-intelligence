// Custom save dialog component for Tauri environments with directory picker
// Allows users to select a directory and specify filename before saving transcript
import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

interface SaveTranscriptDialogProps {
  open: boolean;
  defaultFileName: string;
  content: string;
  onClose: () => void;
  onSaved?: (fullPath: string) => void;
}

const ensureTxt = (name: string) => (name.endsWith('.txt') ? name : `${name}.txt`);

export function SaveTranscriptDialog({ open: isOpen, defaultFileName, content, onClose, onSaved }: SaveTranscriptDialogProps) {
  const [directory, setDirectory] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string>(ensureTxt(defaultFileName));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const pickDirectory = async () => {
    setError(null);
    try {
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === 'string') {
        setDirectory(dir);
      }
    } catch (e: any) {
      setError(e?.message ?? 'Failed to open directory picker');
    }
  };

  const handleSave = async () => {
    setError(null);
    if (!directory) {
      setError('Please choose a folder.');
      return;
    }
    const safeName = ensureTxt(fileName.trim() || defaultFileName);
    const fullPath = `${directory.replace(/\/$/, '')}/${safeName}`;
    setSaving(true);
    try {
      await writeTextFile(fullPath, content);
      onSaved?.(fullPath);
      onClose();
    } catch (e: any) {
      setError(e?.message ?? 'Failed to save file');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{
      position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
    }}>
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
          <div style={{ marginBottom: 12 }}>
            <label style={{ display: 'block', fontSize: 12, color: '#555', marginBottom: 6 }}>Folder</label>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                readOnly
                value={directory ?? ''}
                placeholder="No folder selected"
                style={{ flex: 1, padding: '10px 12px', border: '1px solid #ddd', borderRadius: 8, background: '#fafafa' }}
              />
              <button onClick={pickDirectory} style={{ padding: '10px 14px', borderRadius: 8, border: '1px solid #ddd', background: '#f5f5f5' }}>
                Choose…
              </button>
            </div>
          </div>
          {error && (
            <div style={{ color: '#b00020', fontSize: 13, marginTop: 8 }}>{error}</div>
          )}
        </div>
        <div style={{ padding: 16, display: 'flex', justifyContent: 'flex-end', gap: 8, borderTop: '1px solid #eee' }}>
          <button onClick={onClose} disabled={saving} style={{ padding: '10px 14px', borderRadius: 8, border: '1px solid #ddd', background: '#f5f5f5' }}>Cancel</button>
          <button onClick={handleSave} disabled={saving} style={{ padding: '10px 14px', borderRadius: 8, border: '1px solid #0b5', background: '#0c6', color: '#fff' }}>{saving ? 'Saving…' : 'Save'}</button>
        </div>
      </div>
    </div>
  );
}