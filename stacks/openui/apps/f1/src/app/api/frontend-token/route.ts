export function POST() { return Response.json({error:"Cloud storage is disabled."},{status:410}); }
export const GET = POST;
