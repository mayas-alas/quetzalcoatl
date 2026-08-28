param(
  [Parameter(Mandatory=$true)][string]$InputText,
  [string]$Model = "auto",
  [string]$BaseUrl = "http://127.0.0.1:31415/v1",
  [string]$ApiKey = $env:FREELLMAPI_API_KEY
)
$ErrorActionPreference = "Stop"
if (-not $ApiKey) { Write-Error "FREELLMAPI_API_KEY no esta definida (o falta -ApiKey)."; exit 1 }
$body = @{ model = $Model; input = $InputText } | ConvertTo-Json -Compress
try {
  $resp = Invoke-RestMethod -Method Post -Uri "$BaseUrl/embeddings" `
    -Headers @{ Authorization = "Bearer $ApiKey" } -ContentType "application/json" -Body $body
  $resp | ConvertTo-Json -Depth 10
} catch {
  $status = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { "?" }
  Write-Error "HTTP $status : $($_.Exception.Message)"
  exit 2
}