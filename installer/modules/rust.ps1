function Build-RustReleaseArtifacts {
    param(
        [Parameter(Mandatory = $true)] [string[]] $Packages,
        [Parameter(Mandatory = $true)] [string[]] $ReleaseBinaries
    )

    Remove-Item -LiteralPath $ReleaseBinaries -Force -ErrorAction SilentlyContinue
    $rustBuildStarted = [DateTime]::UtcNow

    foreach ($rustPackage in $Packages) {
        & cargo rustc --locked --release -p $rustPackage -- -C target-feature=+crt-static
        if ($LASTEXITCODE -ne 0) {
            throw "Static-CRT Rust release build failed for $rustPackage."
        }
    }

    foreach ($releaseBinary in $ReleaseBinaries) {
        if (-not (Test-Path -LiteralPath $releaseBinary)) {
            throw "Fresh Rust release binary is absent: $releaseBinary"
        }
        if ((Get-Item -LiteralPath $releaseBinary).LastWriteTimeUtc -lt $rustBuildStarted.AddSeconds(-2)) {
            throw "Rust release binary was not freshly produced by this build: $releaseBinary"
        }
    }

    & cargo test --locked --release -p gnx-service payload_manifest_matches_all_installed_files
    if ($LASTEXITCODE -ne 0) {
        throw "gnx-service/runtime manifest build contract test failed."
    }
}

function Test-StaticCrtRustArtifacts {
    param(
        [Parameter(Mandatory = $true)] [hashtable[]] $Artifacts
    )

    foreach ($rustBinary in $Artifacts) {
        $prohibitedCrtImports = Get-PeImportedDllNames -Path $rustBinary.Path |
            Where-Object { $_ -match '(?i)^(?:api-ms-win-crt-.+|vcruntime[0-9].*|msvcp[0-9].*|msvcr[0-9].*|concrt[0-9].*|vcomp[0-9].*|ucrtbase)\.dll$' }
        if ($prohibitedCrtImports) {
            throw "$($rustBinary.Name) must not dynamically import a Visual C++ runtime DLL: $($prohibitedCrtImports -join ', ')"
        }
    }
}
