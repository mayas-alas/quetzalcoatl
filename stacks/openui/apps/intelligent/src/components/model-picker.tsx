"use client";
import { useCallback, useEffect, useState } from 'react';
import { withBase } from '../lib/base-path';

const KEY = 'gnx_model';
function readChoice() {
  const found = document.cookie.match(/(?:^|;\s*)gnx_model=([^;]*)/);
  try { return found ? decodeURIComponent(found[1]) : 'entry-stack'; } catch { return 'entry-stack'; }
}
export async function chooseModel(name: string) {
  const response = await fetch(withBase('/api/models'), {cache: 'no-store'});
  if (!response.ok) throw new Error('No se pudieron cargar los modelos');
  const {models} = await response.json();
  if (!Array.isArray(models) || !models.includes(name)) throw new Error(`Modelo no disponible: ${name}`);
  document.cookie = `${KEY}=${encodeURIComponent(name)}; Path=/; SameSite=Strict; Secure`;
  window.dispatchEvent(new Event('gnx-model-change'));
}
export function ModelPicker() {
  const [models, setModels] = useState<string[]>([]);
  const [value, setValue] = useState('entry-stack');
  const [status, setStatus] = useState('');
  const refresh = useCallback(async () => {
    try {
      const response = await fetch(withBase('/api/models'), {cache:'no-store'});
      if (!response.ok) throw new Error();
      const data = await response.json();
      if (!Array.isArray(data.models)) throw new Error();
      setModels(data.models); setValue(readChoice()); setStatus('');
    } catch { setStatus('No se pudieron cargar los modelos'); }
  }, []);
  useEffect(() => { void refresh(); const update = () => setValue(readChoice()); window.addEventListener('gnx-model-change', update); return () => window.removeEventListener('gnx-model-change', update); }, [refresh]);
  return <div style={{display:'flex', gap:8, alignItems:'center', padding:'8px 12px', flexWrap:'wrap'}}>
    <label>Modelo <select aria-label="Modelo" value={models.includes(value) ? value : ''} onChange={async e => { try { await chooseModel(e.target.value); setStatus(''); } catch { setStatus('No se pudo seleccionar el modelo'); } }}>
      {!models.includes(value) && <option value="">Seleccionar modelo</option>}
      {models.map(m => <option key={m} value={m}>{m}</option>)}
    </select></label>
    <button type="button" onClick={() => void refresh()}>Actualizar modelos</button>
    <span role="status">{status}</span>
  </div>;
}
