[CmdletBinding()]
param([Parameter(Mandatory)][string]$Nameserver)
$ErrorActionPreference='Stop'
$results=@()
foreach($name in @('app.gnx','compute.gnx')) {
 $normal=@(Resolve-DnsName $name -Type A -DnsOnly | Where-Object Type -eq 'A' | Select-Object -ExpandProperty IPAddress)
 if ($normal -notcontains $Nameserver) {throw "SPLIT_DNS_FAILED: $name"}
 foreach($tcp in @($false,$true)) {
  $answer=@(Resolve-DnsName $name -Server $Nameserver -Type A -DnsOnly -TcpOnly:$tcp | Where-Object Type -eq 'A' | Select-Object -ExpandProperty IPAddress)
  if ($answer -notcontains $Nameserver) {throw "DIRECT_DNS_FAILED: $name tcp=$tcp"}
 }
 # Private CAs have no public revocation endpoint. This retains chain and hostname validation.
 $http=& curl.exe --ssl-revoke-best-effort --noproxy '*' --max-time 15 --silent --show-error --output NUL --write-out '%{http_code}' "https://$name/"
 if ($LASTEXITCODE -ne 0 -or $http -ne '200') {throw "HTTPS_FAILED: $name HTTP=$http"}
 $results+=@{hostname=$name;split_dns=$normal;udp='PASS';tcp='PASS';https=200}
}
@{schema=1;utc=[DateTime]::UtcNow.ToString('o');nameserver=$Nameserver;checks=$results} | ConvertTo-Json -Depth 6
