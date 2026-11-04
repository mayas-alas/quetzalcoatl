import OpenAI from "openai";
import type {
  ChatCompletionChunk,
} from "openai/resources/chat/completions";
import { z } from "zod/v4";
import { f1PromptContext } from "../../../lib/f1/context";
import { f1Executors, f1FunctionTools } from "../../../lib/f1/tools";
import { analyticsPrompt } from "../../../lib/prompt";
import { runChatToolLoop } from "../../../lib/tool-loop";
import { hasAllowedOrigin } from "../../../lib/same-origin";
import { selectedModel } from "../../../lib/gnx-models";

export const runtime = "nodejs";

// Browser-local history is untrusted: accept only bounded conversational text.
const chatRequestSchema = z.object({
  threadId: z.string().max(256).regex(/^[A-Za-z0-9][A-Za-z0-9_-]*$/),
  messages: z.array(z.object({
    role: z.enum(["user", "assistant"]),
    content: z.string().max(60000),
  })).min(1).max(40),
}).refine(v => v.messages.at(-1)?.role === "user" && (v.messages.at(-1)?.content.trim().length ?? 0) > 0 && (v.messages.at(-1)?.content.length ?? 601) <= 600);

async function parseChatRequest(request: Request) {
  if (request.headers.get("content-type")?.split(";")[0].trim() !== "application/json")
    throw new Error("Send JSON.");
  const reader = request.body?.getReader();
  if (!reader) throw new Error("Send a question.");
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > 1_000_000) {
        await reader.cancel();
        throw new Error("Request too large.");
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  return chatRequestSchema.parse(JSON.parse(Buffer.concat(chunks).toString("utf8")));
}

export async function POST(request: Request) {
  // Authentication is enforced at the private ingress; Origin is an additional browser check.
  if (!hasAllowedOrigin(request))
    return Response.json(
      { error: "This example only accepts requests from its local chat interface." },
      { status: 403 },
    );
  let body;
  try {
    body = await parseChatRequest(request);
  } catch {
    return Response.json(
      { error: "Send a threadId and the thread's messages, ending with a question of up to 600 characters." },
      { status: 400 },
    );
  }
  const { messages: history } = body;
  const { LLM_BASE_URL: baseURL, LLM_API_KEY: apiKey } = process.env;
  if (!baseURL || !apiKey)
    return Response.json({ error: "Configure OmniRoute endpoint, API key and model on the server." }, { status: 503 });
  let model: string;
  try { model = await selectedModel(request); }
  catch { return Response.json({error:'Modelo no disponible o fallo de OmniRoute.'}, {status:503}); }
  const gateway = new OpenAI({ apiKey, baseURL, maxRetries: 0, timeout: 180000 });
  const tools = f1FunctionTools();
  const functionTools = f1Executors();
  const context = await f1PromptContext(AbortSignal.any([request.signal, AbortSignal.timeout(180000)]));
  const params = {
    model,
    messages: [
      {
        role: "system" as const,
        content: analyticsPrompt({ today: new Date().toISOString().slice(0, 10), ...context }),
      },
      ...history,
    ],
    tools,
    max_completion_tokens: 6000,
  };

  // Open the first round before responding, so a rejected key, rate limit, or unknown model
  // returns as an HTTP error with Gateway's status instead of an event mid-stream.
  let firstStream: AsyncIterable<ChatCompletionChunk>;
  try {
    firstStream = await gateway.chat.completions.create(
      { ...params, stream: true },
      { signal: AbortSignal.any([request.signal, AbortSignal.timeout(180000)]) },
    );
  } catch (error) {
    const upstream = error as { status?: number; message?: string };
    return Response.json(
      { error: upstream.message ?? "OpenUI Gateway request failed." },
      { status: upstream.status ?? 502 },
    );
  }

  // Stream AG-UI events to the browser, running app-owned tools between model turns.
  const encoder = new TextEncoder();
  let open = true;
  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      // Stop sending once the browser disconnects.
      const emit = (event: Record<string, unknown>) => {
        if (open) controller.enqueue(encoder.encode(`data: ${JSON.stringify(event)}\n\n`));
      };
      try {
        await runChatToolLoop({
          client: gateway,
          params,
          firstStream,
          tools: functionTools,
          emit,
          signal: AbortSignal.any([request.signal, AbortSignal.timeout(180000)]),
          maxRounds: 5,
        });
      } catch (error) {
        emit({
          type: "RUN_ERROR",
          message: error instanceof Error ? error.message : String(error),
        });
      } finally {
        if (open) controller.close();
      }
    },
    cancel() {
      open = false;
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
    },
  });
}
