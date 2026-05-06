[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$PreferredMajorMinor = "3.12",
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$PythonArgs
)

$ErrorActionPreference = "Stop"

function Get-PythonVersionObject {
    param([string]$Value)

    if (-not $Value) {
        return $null
    }

    if ($Value -match "(?<version>\d+\.\d+(?:\.\d+)?)") {
        try {
            return [Version]$matches["version"]
        } catch {
            return $null
        }
    }

    return $null
}

function Resolve-PythonPath {
    param([Version]$PreferredVersion)

    $pythonCommand = Get-Command python.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($pythonCommand) {
        return $pythonCommand.Source
    }

    $pyCommand = Get-Command py.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($pyCommand) {
        $rawCandidates = & $pyCommand.Source -0p 2>$null
        if ($LASTEXITCODE -eq 0 -and $rawCandidates) {
            $candidates = foreach ($line in $rawCandidates) {
                if ($line -match "^\s*-V:(?<tag>\S+)(?:\s+\*)?\s+(?<path>[A-Za-z]:\\.+)$") {
                    $path = $matches["path"].Trim()
                    if (Test-Path $path) {
                        [PSCustomObject]@{
                            Version = Get-PythonVersionObject -Value $matches["tag"]
                            Path = $path
                        }
                    }
                }
            }

            $preferredMatch = $candidates |
                Where-Object {
                    $_.Version -and
                    $_.Version.Major -eq $PreferredVersion.Major -and
                    $_.Version.Minor -eq $PreferredVersion.Minor
                } |
                Sort-Object Version -Descending |
                Select-Object -First 1

            if ($preferredMatch) {
                return $preferredMatch.Path
            }

            $fallback = $candidates |
                Where-Object { $_.Version } |
                Sort-Object Version -Descending |
                Select-Object -First 1

            if ($fallback) {
                return $fallback.Path
            }
        }
    }

    $localSearchRoots = @(
        (Join-Path $env:LOCALAPPDATA "Programs\Python"),
        (Join-Path $env:APPDATA "uv\python")
    )

    foreach ($root in $localSearchRoots) {
        if (-not (Test-Path $root)) {
            continue
        }

        $match = Get-ChildItem -Path $root -Recurse -Filter python.exe -ErrorAction SilentlyContinue |
            ForEach-Object {
                [PSCustomObject]@{
                    Path = $_.FullName
                    Version = Get-PythonVersionObject -Value $_.FullName
                }
            } |
            Sort-Object Version -Descending |
            Select-Object -First 1

        if ($match) {
            return $match.Path
        }
    }

    throw "No usable Python runtime was found. Install Python $PreferredMajorMinor or make one discoverable through the Python launcher."
}

if (-not $PythonArgs -or $PythonArgs.Count -eq 0) {
    throw "invoke-python.ps1 requires Python arguments to forward."
}

$preferredVersion = [Version]$PreferredMajorMinor
$pythonPath = Resolve-PythonPath -PreferredVersion $preferredVersion

& $pythonPath @PythonArgs
exit $LASTEXITCODE
