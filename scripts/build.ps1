param(
    [switch]$Package,
    [switch]$CheckpointOnly,
    [switch]$Resume,
    [ValidateRange(0,720)][int]$BudgetMinutes = 0,
    [DateTime]$StartedUtc = [DateTime]::MinValue,
    [string]$Goal,
    [string]$Verified,
    [string]$NextAction,
    [string]$PendingOperation
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$checkpointPath = Join-Path $repo 'build/resume.json'
$boot = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('o')
$ownerStarted = (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString('o')
$state = $null
if (Test-Path -LiteralPath $checkpointPath) {
    if ((Get-Item -LiteralPath $checkpointPath).Length -gt 32768) { throw 'Checkpoint too large' }
    $state = Get-Content -LiteralPath $checkpointPath -Raw | ConvertFrom-Json
    if ($state.schema -ne 1 -or $state.repo -ne $repo) { throw 'Foreign checkpoint; inspect without adopting it' }
}
if ($null -eq $state -or $BudgetMinutes -gt 0) {
    $state = [pscustomobject]@{ schema=1; repo=$repo; head=''; goal=''; last_verified=''; next_action=''; pending=$null; elapsed_seconds=0; budget_seconds=($BudgetMinutes*60); updated_utc=[DateTime]::UtcNow.ToString('o'); boot=$boot; native_logs=@('SCM: GnXCore','journalctl -u gnx-core.target') }
    if ($StartedUtc -ne [DateTime]::MinValue) { $state.elapsed_seconds=[int][Math]::Max(0,([DateTime]::UtcNow-$StartedUtc.ToUniversalTime()).TotalSeconds) }
}
function Save-Checkpoint {
    $now = [DateTime]::UtcNow
    if ($state.boot -eq $boot) { $state.elapsed_seconds += [int][Math]::Max(0,($now-[DateTime]::Parse($state.updated_utc).ToUniversalTime()).TotalSeconds) }
    $state.updated_utc = $now.ToString('o')
    $state.boot = $boot
    $state.head = (git -C $repo rev-parse HEAD)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot checkpoint unknown Git state' }
    $directory = Split-Path -Parent $checkpointPath
    New-Item -ItemType Directory -Force $directory | Out-Null
    $temporary = "$checkpointPath.new"
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($state | ConvertTo-Json -Depth 6))
    $stream = [IO.FileStream]::new($temporary,[IO.FileMode]::Create,[IO.FileAccess]::Write,[IO.FileShare]::None,4096,[IO.FileOptions]::WriteThrough)
    try { $stream.Write($bytes,0,$bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
    if (Test-Path -LiteralPath $checkpointPath) { [IO.File]::Replace($temporary,$checkpointPath,[NullString]::Value) } else { [IO.File]::Move($temporary,$checkpointPath) }
}
function Native([string]$Operation, [scriptblock]$Command) {
    $state.pending = [pscustomobject]@{ operation=$Operation; owner_pid=$PID; owner_started_utc=$ownerStarted; boot=$boot; state='pending' }
    Save-Checkpoint
    & $Command
    if ($LASTEXITCODE -ne 0) { $state.pending.state='failed'; Save-Checkpoint; throw "$Operation failed ($LASTEXITCODE)" }
    $state.last_verified = "$Operation : native exit 0"
    $state.pending = $null
    Save-Checkpoint
}
Push-Location $repo
try {
    if ($Resume) {
        $head = git rev-parse HEAD
        $pendingAlive = $false
        if ($null -ne $state.pending -and $state.pending.boot -eq $boot) {
            $process = Get-Process -Id $state.pending.owner_pid -ErrorAction SilentlyContinue
            $pendingAlive = $null -ne $process -and $process.StartTime.ToUniversalTime().ToString('o') -eq $state.pending.owner_started_utc
        }
        $state
        "CheckpointMatchesHead=$($state.head -eq $head); PendingOwnerAlive=$pendingAlive"
        git status --short
        Get-Service GnXCore -ErrorAction SilentlyContinue | Select-Object Name,Status
        'Read-only resume: reconcile pending native operations before repeating them.'
        return
    }
    if ($Goal) { $state.goal=$Goal }
    if ($Verified) { $state.last_verified=$Verified }
    if ($NextAction) { $state.next_action=$NextAction }
    if ($PendingOperation) { $state.pending=[pscustomobject]@{ operation=$PendingOperation; owner_pid=$PID; owner_started_utc=$ownerStarted; boot=$boot; state='unknown-until-native-check' } }
    if ($CheckpointOnly) { Save-Checkpoint; return }
    Native 'Rust release build' { cargo build --release --locked }
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $version = ($metadata.packages | Where-Object name -eq 'gnx').version
    if ((& './target/release/gnx.exe' --version) -ne "gnx $version") { throw 'Release version mismatch' }
    $release = Join-Path $repo "releases/v$version"
    if (-not (Test-Path -LiteralPath (Join-Path $release 'CHANGELOG.md') -PathType Leaf)) { throw 'Release changelog missing' }
    $releaseBinary = Join-Path $release 'gnx.exe'
    $sourceHash = (Get-FileHash target/release/gnx.exe).Hash
    if (Test-Path -LiteralPath $releaseBinary) {
        if ($sourceHash -ne (Get-FileHash -LiteralPath $releaseBinary).Hash) { throw 'Versioned binary differs; increment version' }
    } else { Copy-Item -LiteralPath 'target/release/gnx.exe' -Destination $releaseBinary }
    if ($sourceHash -ne (Get-FileHash -LiteralPath $releaseBinary).Hash) { throw 'Release binary mismatch' }
    if (-not $Package) { return }
    $head = git rev-parse --short=12 HEAD
    $stage = Join-Path $repo "build/package/v$version-$head/payload"
    New-Item -ItemType Directory -Force $stage,(Join-Path $stage 'images') | Out-Null
    Copy-Item -Recurse -Force config $stage
    New-Item -ItemType Directory -Force (Join-Path $stage 'scripts') | Out-Null
    Copy-Item scripts/*.run (Join-Path $stage 'scripts') -Force
    $inputs = @{ 'rootfs.tar' = 'build/payload/rootfs.tar' }
    foreach ($role in 'gateway','dns','mesh','compute') { $inputs["images/$role.oci.tar"] = "build/payload/images/$role.oci.tar" }
    foreach ($relative in $inputs.Keys) {
        $source = Join-Path $repo $inputs[$relative]
        $destination = Join-Path $stage $relative
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Offline input missing: $source" }
        if (-not (Test-Path -LiteralPath $destination)) { New-Item -ItemType HardLink -Path $destination -Target $source | Out-Null }
        if ((Get-FileHash $source).Hash -ne (Get-FileHash $destination).Hash) { throw "Stale staged input: $relative" }
    }
    $entries = @(Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object Name -ne 'manifest.json' | Sort-Object FullName | ForEach-Object {
        @{ path=$_.FullName.Substring($stage.Length+1).Replace('\','/'); sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    $manifest = Join-Path $stage 'manifest.json'
    [IO.File]::WriteAllText($manifest,(@{version=1;architecture='x86_64';files=$entries}|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
    Native 'Offline payload verification' { & './target/release/gnx.exe' --verify-package $stage }
    $output = Join-Path $repo "build/releases/v$version/"
    New-Item -ItemType Directory -Force $output | Out-Null
    Native 'MSI build' { dotnet build installer/GnxCore.wixproj -c Release "-p:OutputPath=$output" "-p:GnxVersion=$version" "-p:GnxPayload=$stage" }
    $windowsInstaller = New-Object -ComObject WindowsInstaller.Installer
    $database = $windowsInstaller.OpenDatabase((Join-Path $output 'GnxCore.msi'),0)
    $query = $database.OpenView("SELECT HashPart1,HashPart2,HashPart3,HashPart4 FROM MsiFileHash WHERE File_='CoreExe'")
    $query.Execute(); $record=$query.Fetch()
    $binaryHash=$windowsInstaller.FileHash((Join-Path $repo 'target/release/gnx.exe'),0)
    if ($null -eq $record) { throw 'MSI binary hash missing' }
    foreach ($i in 1..4) { if ($record.IntegerData($i) -ne $binaryHash.IntegerData($i)) { throw 'MSI binary mismatch; refusing bundle/release' } }
    [Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null
    Native 'EXE bundle build' { dotnet build installer/bundle/Bundle.wixproj -c Release "-p:OutputPath=$output" "-p:GnxVersion=$version" }
    Copy-Item target/release/gnx.exe $output
} finally { Pop-Location }
