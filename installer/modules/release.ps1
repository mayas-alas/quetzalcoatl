function Read-ReleaseManifest {
    param([Parameter(Mandatory = $true)][string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Release manifest is absent: $Path"
    }

    $facts = @{}
    $section = ''
    foreach ($rawLine in (Get-Content -LiteralPath $Path -Encoding utf8)) {
        $line = $rawLine.Trim()
        if ($line.Length -eq 0 -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[(?<section>[A-Za-z0-9_.-]+)\]$') {
            $section = $Matches.section
            continue
        }
        if ($line -notmatch '^(?<name>[A-Za-z0-9_]+)\s*=\s*(?<value>.+)$') {
            throw "Unsupported release manifest syntax: $rawLine"
        }
        $name = if ($section) { "$section.$($Matches.name)" } else { $Matches.name }
        $literal = $Matches.value.Trim()
        if ($literal -match '^"(?<text>.*)"$') {
            $value = $Matches.text
        } elseif ($literal -match '^-?[0-9]+$') {
            $value = [int64] $literal
        } elseif ($literal -eq 'true' -or $literal -eq 'false') {
            $value = $literal -eq 'true'
        } else {
            throw "Unsupported release manifest value for ${name}: $literal"
        }
        if ($facts.ContainsKey($name)) {
            throw "Duplicate release manifest fact: $name"
        }
        $facts[$name] = $value
    }
    return $facts
}

function Get-ReleaseFact {
    param(
        [Parameter(Mandatory = $true)] [hashtable] $Facts,
        [Parameter(Mandatory = $true)] [string] $Name
    )
    if (-not $Facts.ContainsKey($Name)) {
        throw "Required release fact is absent: $Name"
    }
    return $Facts[$Name]
}
