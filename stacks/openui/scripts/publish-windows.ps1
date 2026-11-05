$ErrorActionPreference='Stop'
$principal=[Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {throw 'Ejecutar como administrador'}
$path=Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
$marker='# GnXCore:7a34f3dc-ea24-4ddf-8adb-5817bf28bd21'
$legacy='127.0.0.1 app.gnx compute.gnx pocket.gnx rustfs.gnx s3.gnx netbird.gnx '+$marker
$current='127.0.0.1 app.gnx compute.gnx pocket.gnx rustfs.gnx s3.gnx netbird.gnx f1.gnx intelligent.gnx omniroute.gnx api.gnx '+$marker
$alias='127.0.0.1 api.gnx # GnXOpenUI:7a34f3dc-ea24-4ddf-8adb-5817bf28bd21'
$text=[IO.File]::ReadAllText($path)
$lines=@($text -split '\r?\n')
if ($lines -notcontains $current) {throw 'Entrada propia no reconocida; no se modifica hosts'}
foreach ($line in $lines) {
  $parts=@(($line.Split('#')[0].Trim()) -split '\s+')
  if ($parts.Count -gt 1 -and ($parts | Select-Object -Skip 1) -contains 'api.gnx' -and $parts[0] -ne '127.0.0.1') {throw 'api.gnx tiene una direccion distinta'}
}
$lines=@($lines | Where-Object {$_ -ne $legacy})
if ($lines -notcontains $alias) {$lines += $alias}
$updated=($lines -join [Environment]::NewLine).TrimEnd()+[Environment]::NewLine
if ($updated -ne $text) {[IO.File]::WriteAllText($path,$updated,[Text.UTF8Encoding]::new($false))}
Clear-DnsClientCache
$addresses=[System.Net.Dns]::GetHostAddresses('api.gnx')
if (-not ($addresses | Where-Object IPAddressToString -eq '127.0.0.1')) {throw 'Alias local no confirmado'}
'api.gnx -> 127.0.0.1'
