import http from 'node:http';
import { Readable } from 'node:stream';
const base = process.env.LLM_BASE_URL || 'http://host.containers.internal:20128/v1';
const key = process.env.LLM_API_KEY;
const model = process.env.LLM_MODEL || 'gnx-openui';

let active = 0;
http.createServer(async (req, res) => {
  if (req.method === 'GET' && req.url === '/health') {
    res.writeHead(200).end('ok'); return;
  }
  if (req.method === 'GET' && (req.url === '/v1/model/info' || req.url === '/v1/models')) {
    if (!key) { res.writeHead(503).end('Configure protected OmniRoute client settings'); return; }
    try {
      const upstream = await fetch(base + '/models', {
        headers: {Authorization:'Bearer ' + key},
        signal: AbortSignal.timeout(15000),
      });
      if (!upstream.ok) { res.writeHead(502).end('OmniRoute model discovery failed'); return; }
      const result = await upstream.json();
      if (!Array.isArray(result.data)) throw new Error('Invalid model list');
      const models = result.data.filter(item => item && typeof item.id === 'string' && (item.id === 'auto/coding:free' || (item.owned_by === 'combo' && item.id !== 'auto' && !item.id.startsWith('auto/'))));
      res.setHeader('Content-Type', 'application/json');
      res.setHeader('Cache-Control', 'no-store');
      if (req.url === '/v1/model/info') {
        res.end(JSON.stringify({data: models.map(item => ({
          model_name: item.id, litellm_params: {model: item.id},
        }))}));
      } else {
        res.end(JSON.stringify({object:'list', data:models}));
      }
    } catch {
      res.writeHead(502).end('OmniRoute model discovery failed');
    }
    return;
  }
  if (req.method !== 'POST' || req.url !== '/v1/chat/completions') {
    res.writeHead(404).end(); return;
  }
  if (!key) { res.writeHead(503).end('Configure protected OmniRoute client settings'); return; }
  if (active >= 2) { res.writeHead(429).end('Busy'); return; }
  active++;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 180000);
  res.on('close', () => { if (!res.writableFinished) controller.abort(); });
  try {
    let size = 0; const chunks = [];
    for await (const chunk of req) {
      size += chunk.length;
      if (size > 16 * 1024 * 1024) { res.writeHead(413).end(); return; }
      chunks.push(chunk);
    }
    const body = JSON.parse(Buffer.concat(chunks));
    body.model = body.model || model;
    delete body.reasoning_effort;
    delete body.reasoning;
    body.max_tokens = Math.min(Number(body.max_tokens) || 8192, 8192);
    const upstream = await fetch(base + '/chat/completions', {
      method: 'POST', signal:controller.signal,
      headers:{'Content-Type':'application/json',Authorization:'Bearer ' + key},
      body:JSON.stringify(body),
    });
    res.writeHead(upstream.status, {
      'Content-Type':upstream.headers.get('content-type') || 'application/json',
      'Cache-Control':'no-store',
    });
    if (upstream.body) {
      const stream = Readable.fromWeb(upstream.body);
      stream.on('error', () => res.destroy());
      await new Promise((resolve, reject) => {
        res.once('finish', resolve); res.once('close', resolve);
        stream.once('error', reject); stream.pipe(res);
      });
    } else res.end();
  } catch {
    if (!res.headersSent) res.writeHead(502).end('OmniRoute request failed');
    else res.destroy();
  } finally { clearTimeout(timer); active--; }
}).listen(8080, '0.0.0.0');
