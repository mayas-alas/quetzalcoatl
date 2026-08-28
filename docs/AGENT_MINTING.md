# Minting Agents with Embedding over Pi (FreeLLMAPI)

## Prerequisites

1. FreeLLMAPI running at `http://127.0.0.1:31415/v1`
2. API key set in `FREELLMAPI_API_KEY` (format: `freellmapi-a0404d7b6b797be6408109b293c920c90b919317acda57dd`)
3. Pi agent files in `.kilo/agent/`

## Minting Process

### 1. Create Agent Definition (`.kilo/agent/<name>.md`)

Use YAML frontmatter with required fields:

```yaml
---
description: <Spanish description>
mode: primary | subagent | all
model: <provider/model>
color: "<hex-color>"
---
```

### 2. Add Embedding Instructions

For OpenAI-compatible agents (pi-open):

- Endpoint: `POST /v1/embeddings` with model `auto`
- CLI helper: `tools/pi-embed.ps1`
- Auth: Bearer `FREELLMAPI_API_KEY`

For Claude-compatible agents (pi-claude):

- Same embeddings endpoint works via FreeLLMAPI
- Use `tools/pi-embed.ps1` directly

### 3. Validate

- Edit the repo via Kilo editor
- Check agent appears in `/agents` command palette

## Existing Agents

| Agent | Mode | Default Model | Embedding |
|-------|------|---------------|-----------|
| pi-open | primary | openai/auto | /v1/embeddings |
| pi-claude | primary | anthropic/auto | /v1/embeddings |
| pi | primary | openai/auto | /v1/embeddings |
| pi-embeddings | subagent | openai/auto | /v1/embeddings |

## Smoke Test

```powershell
$body = @{ model = "bge-m3"; input = "test" } | ConvertTo-Json -Compress
$h = @{ Authorization = "Bearer $env:FREELLMAPI_API_KEY" }
Invoke-RestMethod -Method POST -Uri 'http://127.0.0.1:31415/v1/embeddings' -Headers $h -Body $body -ContentType 'application/json'
```

Expected: `data.embedding` with token usage.