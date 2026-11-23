$ErrorActionPreference = 'Stop'
# This is the only automatic release write: create a new unpublished candidate.
# No update/delete, tag push, clobber, or publication command belongs in this path.
if ($env:GITHUB_EVENT_NAME -cne 'push' -or $env:GITHUB_REF -cne 'refs/heads/main' -or $env:CLI_MODE -cne 'preview') {
    throw 'Preview candidates require a push to main and the isolated source preview'
}
if ($env:GITHUB_SHA -cnotmatch '^[0-9a-f]{40}$' -or $env:GITHUB_RUN_ID -cnotmatch '^[0-9]+$' -or
    $env:GITHUB_RUN_ATTEMPT -cnotmatch '^[1-9][0-9]*$' -or $env:GNX_VERSION -cnotmatch '^\d+\.\d+\.\d+$' -or
    $env:GITHUB_REPOSITORY -cnotmatch '^[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+$') { throw 'Invalid candidate provenance' }
$head = git rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $head -cne $env:GITHUB_SHA) { throw 'Candidate source commit differs from the pushed commit' }
$output = Join-Path $PSScriptRoot "../build/previews/v$env:GNX_VERSION"
$installers = Join-Path $output 'installers'
$canonical = Get-Content (Join-Path $output 'build-provenance.json') -Raw | ConvertFrom-Json
if ($canonical.source_commit -cne $env:GITHUB_SHA -or $canonical.version -cne $env:GNX_VERSION -or
    $canonical.source_tree_dirty -ne $false -or $canonical.cli_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
    $canonical.preview -ne $true -or $canonical.compiled_from_source -ne $true) {
    throw 'Candidate requires provenance from a source preview of the pushed commit'
}
$names = @('GnxCore.msi', 'GnxCoreSetup.exe', 'SHA256SUMS.txt')
$sums = @(Get-Content -LiteralPath (Join-Path $installers 'SHA256SUMS.txt'))
if ($sums.Count -ne 2) { throw 'Preview checksums must contain exactly two installers' }
$files = @{}
foreach ($name in $names) {
    $file = Get-Item -LiteralPath (Join-Path $installers $name)
    if ($file.Length -le 0 -or $file.Length -ge 2GB -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Invalid preview asset (must be a regular file below GitHub's 2 GiB per-asset limit): $name"
    }
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($name -ne 'SHA256SUMS.txt' -and @($sums | Where-Object { $_ -ceq "$hash  $name" }).Count -ne 1) { throw "Preview checksum mismatch: $name" }
    $files[$name] = @{ path=$file.FullName; bytes=$file.Length; sha256=$hash }
}
$candidate = "ci-main-$env:GITHUB_SHA-$env:GITHUB_RUN_ID-$env:GITHUB_RUN_ATTEMPT"
$endpoint = "repos/$env:GITHUB_REPOSITORY/releases"
# Read all pages so a collision cannot hide behind older releases. Never adopt a
# pre-existing or partially uploaded candidate, even if a previous write timed out.
$existing = @(gh api --paginate "$endpoint`?per_page=100" --jq '.[].tag_name')
if ($LASTEXITCODE -ne 0) { throw 'Cannot check existing candidates; nothing created' }
if ($candidate -cin $existing) { throw "Candidate $candidate already exists; inspect it and rerun as a new attempt, without replacing it" }
$refs = @(gh api "repos/$env:GITHUB_REPOSITORY/git/matching-refs/tags/$candidate" --jq '.[].ref')
if ($LASTEXITCODE -ne 0 -or $refs.Count -ne 0) { throw 'Candidate tag name is unavailable; no release created' }
$run = "$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID"
$notes = "Unpublished source preview; not a canonical versioned delivery or runtime acceptance.`n`nCommit: $env:GITHUB_SHA`nVersion: $env:GNX_VERSION`nRun: $run (attempt $env:GITHUB_RUN_ATTEMPT)`nCLI SHA256: $($canonical.cli_sha256)`nOffline inputs: $env:PAYLOAD_RELEASE SHA256 $env:PAYLOAD_SHA256`n`n" + ($sums -join "`n")
$request = @{
    tag_name=$candidate; target_commitish=$env:GITHUB_SHA
    name="GnX Core $env:GNX_VERSION preview $($env:GITHUB_SHA.Substring(0,12)) (run $env:GITHUB_RUN_ID/$env:GITHUB_RUN_ATTEMPT)"
    body=$notes; draft=$true; prerelease=$true; make_latest='false'; generate_release_notes=$false
}
$requestPath = Join-Path $env:RUNNER_TEMP 'preview-release.json'
[IO.File]::WriteAllText($requestPath, ($request | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
$created = gh api --method POST $endpoint --input $requestPath | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $null -eq $created.id) { throw "Candidate creation failed or is uncertain; inspect $candidate before any recovery (no automatic retry)" }
if ($created.draft -ne $true -or $created.prerelease -ne $true -or $created.tag_name -cne $candidate -or $created.target_commitish -cne $env:GITHUB_SHA) {
    throw 'Created candidate metadata differs; inspect the release, no further writes attempted'
}
"Candidate created: $($created.html_url). Upload/verification must complete before using it." >> $env:GITHUB_STEP_SUMMARY
# No --clobber: a failed/ambiguous upload remains visible and fails the run.
gh release upload $candidate $files['GnxCore.msi'].path $files['GnxCoreSetup.exe'].path $files['SHA256SUMS.txt'].path --repo $env:GITHUB_REPOSITORY
if ($LASTEXITCODE -ne 0) { throw "Candidate upload failed; inspect $candidate for partial assets and rerun as a new attempt, without deleting/replacing it" }
$remote = gh api "$endpoint/$($created.id)" | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot verify uploaded candidate' }
if ($remote.draft -ne $true -or $remote.prerelease -ne $true -or $remote.immutable -eq $true -or
    $remote.tag_name -cne $candidate -or $remote.target_commitish -cne $env:GITHUB_SHA -or @($remote.assets).Count -ne 3) {
    throw 'Candidate metadata/assets differ; no publication or replacement attempted'
}
foreach ($name in $names) {
    $asset = @($remote.assets | Where-Object { $_.name -ceq $name })
    if ($asset.Count -ne 1 -or $asset[0].state -cne 'uploaded' -or $asset[0].size -ne $files[$name].bytes -or
        $asset[0].digest -cne "sha256:$($files[$name].sha256)") { throw "Remote candidate size/SHA256 differs: $name" }
}
"Verified unpublished preview: $($remote.html_url). Owners with repository write access can download its MSI, EXE and SHA256SUMS from Releases; public publication requires a separate explicit decision." >> $env:GITHUB_STEP_SUMMARY
