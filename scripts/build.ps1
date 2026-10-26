param(
    [switch]$Package,
    [switch]$UseVersionedBinary,
    [switch]$SealBase,
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
if ($UseVersionedBinary -and (-not $Package -or $SealBase)) {
    throw '-UseVersionedBinary requires -Package and cannot be combined with -SealBase'
}
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
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) { $state.pending.state='failed'; Save-Checkpoint; throw "$Operation failed ($exitCode)" }
    $state.last_verified = "$Operation : native exit 0"
    $state.pending = $null
    Save-Checkpoint
}
function Remove-IntermediateCopy([string]$Intermediate, [string]$VerifiedOutput) {
    if (Test-Path -LiteralPath $Intermediate -PathType Leaf) {
        $item = Get-Item -LiteralPath $Intermediate
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Linked intermediate artifact' }
        if ((Get-FileHash -LiteralPath $Intermediate).Hash -ne (Get-FileHash -LiteralPath $VerifiedOutput).Hash) {
            throw 'Intermediate differs from verified output; nothing removed'
        }
        Remove-Item -LiteralPath $Intermediate
    }
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
    if (-not $UseVersionedBinary) { Native 'Rust release build' { cargo build --release --locked } }
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $version = ($metadata.packages | Where-Object name -eq 'gnx').version
    $release = Join-Path $repo "releases/v$version"
    if (-not (Test-Path -LiteralPath (Join-Path $release 'CHANGELOG.md') -PathType Leaf)) { throw 'Release changelog missing' }
    $releaseBinary = Join-Path $release 'gnx.exe'
    if ($UseVersionedBinary) {
        if (-not (Test-Path -LiteralPath $releaseBinary -PathType Leaf)) { throw 'Versioned CLI missing; build and version it before packaging' }
        if ((& $releaseBinary --version) -ne "gnx $version" -or $LASTEXITCODE -ne 0) { throw 'Versioned CLI version mismatch' }
        New-Item -ItemType Directory -Force (Join-Path $repo 'target/release') | Out-Null
        Copy-Item -LiteralPath $releaseBinary -Destination (Join-Path $repo 'target/release/gnx.exe') -Force
    }
    if ((& './target/release/gnx.exe' --version) -ne "gnx $version" -or $LASTEXITCODE -ne 0) { throw 'Release version mismatch' }
    $sourceHash = (Get-FileHash target/release/gnx.exe).Hash
    if (Test-Path -LiteralPath $releaseBinary) {
        if ($sourceHash -ne (Get-FileHash -LiteralPath $releaseBinary).Hash) { throw 'Versioned binary differs; increment version' }
    } else { Copy-Item -LiteralPath 'target/release/gnx.exe' -Destination $releaseBinary }
    if ($sourceHash -ne (Get-FileHash -LiteralPath $releaseBinary).Hash) { throw 'Release binary mismatch' }
    $output = Join-Path $repo "build/releases/v$version/"
    New-Item -ItemType Directory -Force $output | Out-Null
    if ($SealBase) {
        $crane = Join-Path $repo 'build/tools/crane/crane.exe'
        if (-not (Test-Path $crane)) { throw 'Pinned crane build tool missing' }
        $layer = Join-Path $repo "build/seal/v$version/layer"
        New-Item -ItemType Directory -Force (Join-Path $layer 'etc/systemd/system'),(Join-Path $layer 'usr/share/licenses/gnx') | Out-Null
        Copy-Item config/wsl.conf (Join-Path $layer 'etc/wsl.conf') -Force
        Copy-Item LICENSE (Join-Path $layer 'usr/share/licenses/gnx/LICENSE') -Force
        $layerTar = Join-Path (Split-Path $layer -Parent) 'layer.tar'
        # mtree supplies Unix metadata explicitly instead of inheriting Windows
        # ACL-derived modes into an image that will also run in LXC.
        $mtree = Join-Path (Split-Path $layer -Parent) 'layer.mtree'
        $spec = @('#mtree','./ type=dir mode=0755 uid=0 gid=0 time=0')
        $spec += Get-ChildItem -LiteralPath $layer -Recurse -Force | Sort-Object FullName | ForEach-Object {
            $relative = ('./'+$_.FullName.Substring($layer.Length+1).Replace('\','/')).Replace(' ','\040')
            if ($_.PSIsContainer) { "$relative type=dir mode=0755 uid=0 gid=0 time=0" }
            else { $contents=$_.FullName.Replace('\','/').Replace(' ','\040'); "$relative type=file mode=0644 uid=0 gid=0 time=0 size=$($_.Length) contents=$contents" }
        }
        $spec += Get-Content config/base-masks | ForEach-Object { "./etc/systemd/system/$_ type=link uid=0 gid=0 time=0 link=/dev/null" }
        [IO.File]::WriteAllLines($mtree,$spec,[Text.UTF8Encoding]::new($false))
        Native 'Base image overlay archive' { tar -cf $layerTar "@$mtree" }
        $base = (Get-Content config/bootc.json -Raw | ConvertFrom-Json).source
        $image = Join-Path (Split-Path $layer -Parent) 'base.image.tar'
        $revision = git rev-parse HEAD
        Native 'Sealed base OCI' { & $crane mutate $base --platform=linux/amd64 --append $layerTar --label "org.opencontainers.image.version=$version" --label 'org.opencontainers.image.title=GnX Core Base' --label 'org.opencontainers.image.source=https://github.com/mayas-alas/quetzalcoatl-next' --label "org.opencontainers.image.revision=$revision" --output $image }
        $digest = & $crane digest --tarball $image
        if ($LASTEXITCODE -ne 0 -or $digest -notmatch '^sha256:[0-9a-f]{64}$') { throw 'Sealed image digest unavailable' }
        [IO.File]::WriteAllText((Join-Path $output 'base-image.digest'),$digest,[Text.UTF8Encoding]::new($false))
        $rootfs = Join-Path $repo 'build/payload/rootfs.new.tar'
        Native 'Sealed rootfs export' { & $env:ComSpec /d /s /c "type `"$image`" | `"$crane`" export - `"$rootfs`"" }
        Move-Item -LiteralPath $rootfs -Destination (Join-Path $repo 'build/payload/rootfs.tar') -Force
    }
    if (-not $Package) { return }
    $baseDigestFile = Join-Path $output 'base-image.digest'
    if (-not (Test-Path $baseDigestFile)) { throw 'Seal the versioned base first with -SealBase' }
    $head = git rev-parse --short=12 HEAD
    $stage = Join-Path $repo "build/package/v$version-$head/payload"
    New-Item -ItemType Directory -Force $stage,(Join-Path $stage 'images') | Out-Null
    $runtimeConfig = Join-Path $stage 'config'
    New-Item -ItemType Directory -Force $runtimeConfig | Out-Null
    Get-ChildItem config | Where-Object { $_.Name -notin @('base-masks','bootc.json','base') } | Copy-Item -Recurse -Force -Destination $runtimeConfig
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
    [IO.File]::WriteAllText($manifest,(@{version=1;architecture='x86_64';product_version=$version;source_commit=(git rev-parse HEAD);base_image=([IO.File]::ReadAllText($baseDigestFile));files=$entries}|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
    Native 'Offline payload verification' { & './target/release/gnx.exe' --verify-package $stage }
    Native 'MSI build' { dotnet build installer/GnxCore.wixproj -c Release -t:Rebuild "-p:OutputPath=$output" "-p:GnxVersion=$version" "-p:GnxPayload=$stage" }
    $localizedMsi = Join-Path $output 'en-us/GnxCore.msi'
    $msi = Join-Path $output 'GnxCore.msi'
    if (Test-Path -LiteralPath $localizedMsi) { Move-Item -LiteralPath $localizedMsi -Destination $msi -Force }
    $windowsInstaller = New-Object -ComObject WindowsInstaller.Installer
    $database = $windowsInstaller.OpenDatabase($msi,0)
    $query = $database.OpenView("SELECT HashPart1,HashPart2,HashPart3,HashPart4 FROM MsiFileHash WHERE File_='CoreExe'")
    $query.Execute(); $record=$query.Fetch()
    $binaryHash=$windowsInstaller.FileHash((Join-Path $repo 'target/release/gnx.exe'),0)
    if ($null -eq $record) { throw 'MSI binary hash missing' }
    foreach ($i in 1..4) { if ($record.IntegerData($i) -ne $binaryHash.IntegerData($i)) { throw 'MSI binary mismatch; refusing bundle/release' } }
    $productVersion = $database.OpenView("SELECT Value FROM Property WHERE Property='ProductVersion'")
    $productVersion.Execute(); $versionRecord=$productVersion.Fetch()
    if ($null -eq $versionRecord -or $versionRecord.StringData(1) -ne $version) { throw 'MSI product version mismatch; refusing bundle/release' }
    # Cabinets are embedded in the verified MSI. Discard only the generated
    # loose copies, not any versioned installer, before building the large EXE.
    $media = $database.OpenView('SELECT Cabinet FROM Media')
    $media.Execute()
    while ($cabinet = $media.Fetch()) {
        $name = $cabinet.StringData(1)
        if ($name -notmatch '^#cab[0-9]+\.cab$') { throw 'Unexpected/non-embedded MSI cabinet; nothing removed' }
        $stream = $database.OpenView("SELECT Name FROM _Streams WHERE Name='$($name.Substring(1))'")
        $stream.Execute()
        if ($null -eq $stream.Fetch()) { throw 'Embedded cabinet missing' }
        $stream.Close()
        $loose = Join-Path $repo "installer/obj/Release/en-us/$name"
        if (Test-Path -LiteralPath $loose -PathType Leaf) {
            if ((Get-Item -LiteralPath $loose).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Linked cabinet copy' }
            Remove-Item -LiteralPath $loose
        }
    }
    $media.Close()
    [Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null
    Remove-IntermediateCopy (Join-Path $repo 'installer/obj/Release/en-us/GnxCore.msi') $msi
    Native 'EXE bundle build' { dotnet build installer/bundle/Bundle.wixproj -c Release -t:Rebuild "-p:OutputPath=$output" "-p:GnxVersion=$version" }
    Remove-IntermediateCopy (Join-Path $repo 'installer/bundle/obj/Release/GnxCoreSetup.exe') (Join-Path $output 'GnxCoreSetup.exe')
    Copy-Item target/release/gnx.exe $output
} finally { Pop-Location }
