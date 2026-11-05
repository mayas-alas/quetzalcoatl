const allowed = (m: any) => typeof m?.id === 'string' && (m.id === 'auto/coding:free' || (m.owned_by === 'combo' && m.id !== 'auto' && !m.id.startsWith('auto/')));

export async function modelNames(): Promise<string[]> {
  const { LLM_BASE_URL, LLM_API_KEY } = process.env;
  if (!LLM_BASE_URL || !LLM_API_KEY) throw new Error('OmniRoute no está configurado');
  const response = await fetch(`${LLM_BASE_URL}/models`, { headers: { Authorization: `Bearer ${LLM_API_KEY}` }, cache: 'no-store', signal: AbortSignal.timeout(15000) });
  if (!response.ok) throw new Error('No se pudo consultar OmniRoute');
  const data = await response.json();
  if (!Array.isArray(data.data)) throw new Error('Lista de modelos inválida');
  return [...new Set<string>(data.data.filter(allowed).map((m: {id: string}) => m.id))];
}

export async function selectedModel(req: Request): Promise<string> {
  const match = req.headers.get('cookie')?.match(/(?:^|;\s*)gnx_model=([^;]*)/);
  const selected = match ? decodeURIComponent(match[1]) : 'entry-stack';
  if (!(await modelNames()).includes(selected)) throw new Error('El modelo seleccionado no está disponible');
  return selected;
}
