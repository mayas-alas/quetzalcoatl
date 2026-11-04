import { modelNames } from '../../../lib/gnx-models';
export const dynamic = 'force-dynamic';
export async function GET() {
  try { return Response.json({models: await modelNames()}, {headers: {'Cache-Control': 'no-store'}}); }
  catch { return Response.json({error: 'No se pudieron cargar los modelos'}, {status: 502}); }
}
