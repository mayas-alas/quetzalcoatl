param([switch]$ServicesOnly,[string]$Model='entry-stack')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Net.Http
Add-Type -AssemblyName System.Security
$isAdmin=([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if($isAdmin){
  $access=[IO.File]::ReadAllText('C:\ProgramData\GnX Core\openui-access.txt')
  $demoPassword=[regex]::Match($access,'(?m)^Password: (.+)\r?$').Groups[1].Value.Trim()
  $privateEnv=[IO.File]::ReadAllText('C:\ProgramData\GnX Core\maintenance-openui\omniroute-client.env')
  $apiKey=[regex]::Match($privateEnv,'(?m)^LLM_API_KEY=(.+)\r?$').Groups[1].Value.Trim()
}else{
  $repo='C:\Users\mayas\quetzalcoatl'
  $demoBytes=[Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes((Join-Path $repo '.secret\openui-demo-password.dpapi')),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
  $keyBytes=[Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes((Join-Path $repo '.secret\openui-client.dpapi')),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
  $demoPassword=[Text.Encoding]::UTF8.GetString($demoBytes).Trim()
  $apiKey=[Text.Encoding]::UTF8.GetString($keyBytes).Trim()
  [Array]::Clear($demoBytes,0,$demoBytes.Length)
  [Array]::Clear($keyBytes,0,$keyBytes.Length)
}
if(-not $demoPassword -or -not $apiKey){throw 'Private verification credentials unavailable'}
$client=[Net.Http.HttpClient]::new()
$client.Timeout=[TimeSpan]::FromSeconds(240)
function Call-Gnx([string]$url,[string]$method,[object]$body,[string]$auth,[string]$origin) {
  $request=[Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::new($method),$url)
  if($auth){$request.Headers.Add('Authorization',$auth)}
  if($origin){$request.Headers.Add('Origin',$origin)}
  if($null -ne $body){$request.Content=[Net.Http.StringContent]::new(($body|ConvertTo-Json -Depth 20 -Compress),[Text.Encoding]::UTF8,'application/json')}
  try {
    $response=$client.SendAsync($request).GetAwaiter().GetResult()
    $text=$response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
    [PSCustomObject]@{Code=[int]$response.StatusCode;Text=$text}
  } finally {$request.Dispose()}
}
$basic='Basic '+[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('demo:'+$demoPassword))
try {
  foreach($url in @('https://f1.gnx/','https://intelligent.gnx/','https://agent.gnx/')){
    $r=Call-Gnx $url 'GET' $null $basic ''
    if($r.Code -ne 200){throw ($url+' HTTP '+$r.Code)}
    $url+' HTTP 200'
  }
  $r=Call-Gnx 'https://agent.gnx/api/tasks/models' 'GET' $null $basic ''
  if($r.Code -ne 200){throw 'Agent model integration failed'}
  $agentModels=@($r.Text|ConvertFrom-Json)
  $agentModel=@($agentModels|Where-Object {$_.provider -eq 'proxy' -and $_.name -eq $Model}) | Select-Object -First 1
  if(-not $agentModel){throw 'Agent model integration failed'}
  'AGENT_OMNIROUTE_SELECTOR_OK'
  $ws=[Net.WebSockets.ClientWebSocket]::new()
  $ws.Options.SetRequestHeader('Authorization',$basic)
  $ws.Options.AddSubProtocol('binary')
  $timeout=[Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(25))
  try {
    [void]$ws.ConnectAsync([Uri]'wss://agent.gnx/api/proxy/websockify',$timeout.Token).GetAwaiter().GetResult()
    $buffer=New-Object byte[] 32
    $received=$ws.ReceiveAsync([ArraySegment[byte]]::new($buffer),$timeout.Token).GetAwaiter().GetResult()
    if([Text.Encoding]::ASCII.GetString($buffer,0,$received.Count) -notmatch '^RFB '){throw 'Desktop VNC banner absent'}
    'AGENT_DESKTOP_WSS_RFB_OK'
  } finally {$ws.Abort();$ws.Dispose();$timeout.Dispose()}
  if($ServicesOnly){'AGENT_OPENUI_SERVICES_OK';return}
  $models=Call-Gnx 'https://api.gnx/v1/models' 'GET' $null ('Bearer '+$apiKey) ''
  if($models.Code -ne 200 -or -not (($models.Text|ConvertFrom-Json).data|Where-Object id -eq $agentModel.name)){throw ('Agent model absent from OmniRoute: '+$models.Code)}
  'OMNIROUTE_COMBO_MODEL_OK'
  $r=Call-Gnx 'https://api.gnx/v1/chat/completions' 'POST' @{model=$agentModel.name;messages=@(@{role='user';content='Reply only GNX_OK.'});max_completion_tokens=256} ('Bearer '+$apiKey) ''
  if($r.Code -ne 200){throw ('Inference HTTP '+$r.Code)}
  if((($r.Text|ConvertFrom-Json).choices[0].message.content) -notmatch 'GNX_OK'){throw 'Inference did not return requested acknowledgement'}
  'OMNIROUTE_TEXT_INFERENCE_OK'
  $r=Call-Gnx 'https://f1.gnx/api/chat' 'POST' @{threadId='gnx-acceptance-f1';messages=@(@{role='user';content='Show the race results of the 2024 British Grand Prix. Use the F1 tools and display a compact results table.'})} $basic 'https://f1.gnx'
  if($r.Code -ne 200){throw ('F1 chat HTTP '+$r.Code)}
  if($r.Text -match '"type"\s*:\s*"RUN_ERROR"'){throw 'F1 tool stream reported an error'}
  $events=@(($r.Text -split '\r?\n') | Where-Object {$_ -like 'data: *'} | ForEach-Object {try{$_.Substring(6)|ConvertFrom-Json}catch{}})
  $results=@($events|Where-Object type -eq 'TOOL_CALL_RESULT')
  $successful=@($results|Where-Object {try{$result=$_.content|ConvertFrom-Json; -not $result.error -and $_.content.Length -gt 2}catch{$false}})
  if($r.Text -notmatch 'TOOL_CALL_START' -or $r.Text -notmatch 'TEXT_MESSAGE_END' -or $successful.Count -eq 0){throw 'F1 did not complete a successful tool round and an answer'}
  'F1_TOOL_RESULTS='+$results.Count+' SUCCESSFUL='+$successful.Count
  'F1_REAL_TOOL_STREAM_COMPLETED'
  $r=Call-Gnx 'https://intelligent.gnx/api/chat' 'POST' @{messages=@(@{role='user';content='Create a compact business dashboard with revenue 120000 and three project cards: F1, Agent and OmniRoute.'})} $basic 'https://intelligent.gnx'
  if($r.Code -ne 200 -or $r.Text -notmatch 'response.output_text.delta' -or $r.Text -notmatch 'response.completed' -or $r.Text -match '"type"\s*:\s*"error"'){throw ('Intelligent UI generation failed: HTTP '+$r.Code)}
  'INTELLIGENT_REAL_GENERATION_COMPLETED'
  'AGENT_OPENUI_ACCEPTANCE_OK'
} finally {
  $client.Dispose(); $apiKey=$null; $privateEnv=$null; $demoPassword=$null; $basic=$null; $access=$null
}
