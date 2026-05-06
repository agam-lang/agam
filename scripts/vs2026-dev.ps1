[CmdletBinding(PositionalBinding = $false)]
param(
    [ValidateSet(
        "status",
        "install",
        "doctor",
        "cargo-build-driver",
        "cargo-clean",
        "cargo-rebuild-driver",
        "cargo-check",
        "cargo-test",
        "cargo-fmt-check",
        "build-agam-file",
        "run-agam-file",
        "llvm-smoke"
    )]
    [string]$Task = "status",
    [string]$Path,
    [switch]$Passive
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspaceRoot = Split-Path -Parent $scriptRoot
$targetScript = Join-Path $workspaceRoot "devops\scripts\vs2026-dev.ps1"

if ($PSBoundParameters.ContainsKey("Path")) {
    & $targetScript -Task $Task -Path $Path -Passive:$Passive
} else {
    & $targetScript -Task $Task -Passive:$Passive
}
if ($LASTEXITCODE) {
    exit $LASTEXITCODE
}
