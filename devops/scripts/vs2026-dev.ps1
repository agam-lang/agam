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

$scriptPath = $MyInvocation.MyCommand.Path
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspaceRoot = Split-Path -Parent (Split-Path -Parent $scriptRoot)
$vsConfigPath = Join-Path $workspaceRoot ".vsconfig"
$cargoManifestPath = Join-Path $workspaceRoot "Cargo.toml"
$llvmSmokePath = Join-Path $workspaceRoot "examples\llvm_native_smoke.agam"

function As-Array {
    param([object]$Value)

    if ($null -eq $Value) {
        return @()
    }

    if ($Value -is [System.Array]) {
        return $Value
    }

    return @($Value)
}

function Write-Status {
    param(
        [string]$Kind,
        [string]$Message
    )

    Write-Host ("[{0}] {1}" -f $Kind, $Message)
}

function Resolve-VswherePath {
    $candidate = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $candidate) {
        return $candidate
    }

    throw "vswhere.exe was not found under the Visual Studio Installer path."
}

function Get-VsInstances {
    param([string[]]$Requires = @())

    $vswhere = Resolve-VswherePath
    $arguments = @("-format", "json", "-products", "*")
    foreach ($component in $Requires) {
        $arguments += @("-requires", $component)
    }

    $raw = & $vswhere @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "vswhere.exe failed while querying Visual Studio instances."
    }

    if (-not $raw) {
        return @()
    }

    return As-Array (ConvertFrom-Json ($raw -join [Environment]::NewLine))
}

function Get-PreferredVsInstance {
    $instances = Get-VsInstances | Where-Object {
        $_.productId -ne "Microsoft.VisualStudio.Product.BuildTools"
    }

    if (-not $instances) {
        return $null
    }

    $preferred = $instances |
        Where-Object { $_.catalog.productLine -eq "Dev18" } |
        Sort-Object { [Version]$_.installationVersion } -Descending |
        Select-Object -First 1

    if ($preferred) {
        return $preferred
    }

    return $instances |
        Sort-Object { [Version]$_.installationVersion } -Descending |
        Select-Object -First 1
}

function Test-VsComponentInstalled {
    param(
        [object]$Instance,
        [string]$ComponentId
    )

    if ($null -eq $Instance) {
        return $false
    }

    $matches = Get-VsInstances -Requires @($ComponentId)
    return $matches | Where-Object { $_.installationPath -eq $Instance.installationPath } | Select-Object -First 1
}

function Resolve-VsSetupPath {
    $candidate = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\setup.exe"
    if (Test-Path $candidate) {
        return $candidate
    }

    throw "setup.exe was not found under the Visual Studio Installer path."
}

function Resolve-VcVars64Path {
    param([object]$Instance)

    if ($null -eq $Instance) {
        return $null
    }

    $candidate = Join-Path $Instance.installationPath "VC\Auxiliary\Build\vcvars64.bat"
    if (Test-Path $candidate) {
        return $candidate
    }

    return $null
}

function Resolve-VsLlvmBinDir {
    param([object]$Instance)

    if ($null -eq $Instance) {
        return $null
    }

    $candidates = @(
        (Join-Path $Instance.installationPath "VC\Tools\Llvm\x64\bin"),
        (Join-Path $Instance.installationPath "VC\Tools\Llvm\bin")
    )

    foreach ($candidate in $candidates) {
        if (Test-Path (Join-Path $candidate "clang.exe")) {
            return $candidate
        }
    }

    return $null
}

function Resolve-IncredibuildBuildConsolePath {
    $candidates = @(
        "C:\Program Files (x86)\IncrediBuild\BuildConsole.exe",
        "C:\Program Files (x86)\Incredibuild\BuildConsole.exe",
        "C:\Program Files (x86)\Xoreax\IncrediBuild\BuildConsole.exe"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    return $null
}

function Import-VcVarsEnvironment {
    param([string]$VcVarsPath)

    if (-not $VcVarsPath) {
        return
    }

    $environmentLines = & cmd.exe /d /s /c "`"$VcVarsPath`" >nul && set"
    if ($LASTEXITCODE -ne 0) {
        throw "vcvars64.bat failed while initializing the Visual Studio developer environment."
    }

    foreach ($line in $environmentLines) {
        if ($line -match "^([^=]+)=(.*)$") {
            [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
}

function Prepend-PathEntry {
    param([string]$Entry)

    if (-not $Entry) {
        return
    }

    $currentEntries = @($env:PATH -split ";") | Where-Object { $_ }
    if ($currentEntries -contains $Entry) {
        return
    }

    $env:PATH = "$Entry;$env:PATH"
}

function Initialize-VsToolchainEnvironment {
    param([object]$Instance)

    $vcVarsPath = Resolve-VcVars64Path -Instance $Instance
    Import-VcVarsEnvironment -VcVarsPath $vcVarsPath

    $llvmBinDir = Resolve-VsLlvmBinDir -Instance $Instance
    if ($llvmBinDir) {
        Prepend-PathEntry -Entry $llvmBinDir
        $env:AGAM_LLVM_CLANG = Join-Path $llvmBinDir "clang++.exe"
        $env:CC = Join-Path $llvmBinDir "clang.exe"
        $env:CXX = Join-Path $llvmBinDir "clang++.exe"
    }

    $buildConsole = Resolve-IncredibuildBuildConsolePath
    if ($buildConsole) {
        $env:AGAM_INCREDIBUILD_BUILDCONSOLE = $buildConsole
    }

    if ($Instance) {
        $env:AGAM_VISUAL_STUDIO_PATH = $Instance.installationPath
    }
}

function Get-ResolvedCommand {
    param([string]$Name)

    $command = Get-Command $Name -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) {
        return $command.Source
    }

    return $null
}

function Resolve-PythonRuntime {
    $pythonCommand = Get-ResolvedCommand -Name "python.exe"
    if ($pythonCommand) {
        try {
            $versionOutput = & $pythonCommand --version 2>$null
            if ($LASTEXITCODE -eq 0) {
                return [PSCustomObject]@{
                    Path = $pythonCommand
                    Version = ($versionOutput | Select-Object -First 1)
                }
            }
        } catch {
            # Ignore broken shims and keep probing.
        }
    }

    $pyCommand = Get-ResolvedCommand -Name "py.exe"
    if ($pyCommand) {
        try {
            $versionOutput = & $pyCommand -3 --version 2>$null
            if ($LASTEXITCODE -eq 0) {
                return [PSCustomObject]@{
                    Path = "$pyCommand -3"
                    Version = ($versionOutput | Select-Object -First 1)
                }
            }
        } catch {
            # Ignore broken launchers and keep probing.
        }
    }

    return $null
}

function Invoke-ExternalCommand {
    param(
        [string]$Command,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = $workspaceRoot
    )

    Push-Location $WorkingDirectory
    try {
        & $Command @Arguments
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    if ($exitCode -ne 0) {
        exit $exitCode
    }
}

function Invoke-Cargo {
    param([string[]]$Arguments)

    $cargo = Get-ResolvedCommand -Name "cargo.exe"
    if (-not $cargo) {
        throw "cargo.exe was not found on PATH."
    }

    Invoke-ExternalCommand -Command $cargo -Arguments $Arguments
}

function Show-StatusReport {
    param([object]$Instance)

    Write-Status "ok" ("Workspace root: {0}" -f $workspaceRoot)
    Write-Status "ok" ("VS config: {0}" -f $vsConfigPath)

    if ($Instance) {
        Write-Status "ok" ("Visual Studio: {0} ({1})" -f $Instance.displayName, $Instance.installationPath)
    } else {
        Write-Status "missing" "Visual Studio Community 2026 was not found."
    }

    $llvmBinDir = Resolve-VsLlvmBinDir -Instance $Instance
    if ($llvmBinDir) {
        Write-Status "ok" ("VS LLVM bin: {0}" -f $llvmBinDir)
    } else {
        Write-Status "missing" "Visual Studio LLVM tools are missing."
    }

    $clangPath = Get-ResolvedCommand -Name "clang.exe"
    if ($clangPath) {
        Write-Status "ok" ("clang.exe: {0}" -f $clangPath)
    } else {
        Write-Status "missing" "clang.exe is not reachable after VS toolchain import."
    }

    $cargoPath = Get-ResolvedCommand -Name "cargo.exe"
    if ($cargoPath) {
        Write-Status "ok" ("cargo.exe: {0}" -f $cargoPath)
    } else {
        Write-Status "missing" "cargo.exe is not reachable."
    }

    $pythonRuntime = Resolve-PythonRuntime
    if ($pythonRuntime) {
        Write-Status "ok" ("Python: {0} via {1}" -f $pythonRuntime.Version, $pythonRuntime.Path)
    } else {
        Write-Status "warn" "No usable Python runtime was found. The VS Python workload adds IDE support, but install a current Python runtime separately."
    }

    $buildConsole = Resolve-IncredibuildBuildConsolePath
    if ($buildConsole) {
        Write-Status "ok" ("Incredibuild BuildConsole: {0}" -f $buildConsole)
    } else {
        Write-Status "warn" "Incredibuild BuildConsole.exe was not found."
    }

    $requiredChecks = @(
        @{ Id = "Microsoft.VisualStudio.Component.VC.Llvm.Clang"; Label = "VS LLVM component" },
        @{ Id = "Microsoft.VisualStudio.Component.Windows11SDK.26100"; Label = "Windows 11 SDK 26100" },
        @{ Id = "Microsoft.VisualStudio.Workload.Python"; Label = "VS Python workload" },
        @{ Id = "Microsoft.VisualStudio.Workload.NativeCrossPlat"; Label = "VS Linux/Mac C++ workload" },
        @{ Id = "Microsoft.VisualStudio.Workload.NativeGame"; Label = "VS game C++ workload" },
        @{ Id = "Microsoft.VisualStudio.Workload.VisualStudioExtension"; Label = "VS extension workload" },
        @{ Id = "Component.Incredibuild"; Label = "Incredibuild component" }
    )

    foreach ($check in $requiredChecks) {
        if (Test-VsComponentInstalled -Instance $Instance -ComponentId $check.Id) {
            Write-Status "ok" $check.Label
        } else {
            Write-Status "warn" ("Missing from current VS instance: {0}" -f $check.Label)
        }
    }

    Write-Host ""
    Write-Host "Use the repo config with:"
    Write-Host ("  powershell.exe -ExecutionPolicy Bypass -File ""{0}"" -Task install" -f $scriptPath)
    Write-Host ("  powershell.exe -ExecutionPolicy Bypass -File ""{0}"" -Task doctor" -f $scriptPath)
}

function Install-VsRepoConfiguration {
    param([object]$Instance)

    if (-not (Test-Path $vsConfigPath)) {
        throw ".vsconfig was not found at $vsConfigPath"
    }

    if (-not $Instance) {
        throw "No existing Visual Studio instance was found. Install Community 2026 first, then rerun this script."
    }

    $setupPath = Resolve-VsSetupPath
    $arguments = @(
        "modify",
        "--installPath",
        $Instance.installationPath,
        "--config",
        $vsConfigPath
    )

    if ($Passive) {
        $arguments += "--passive"
    }

    & $setupPath @arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$vsInstance = Get-PreferredVsInstance

if ($Task -eq "install") {
    Install-VsRepoConfiguration -Instance $vsInstance
    exit 0
}

Initialize-VsToolchainEnvironment -Instance $vsInstance

switch ($Task) {
    "status" {
        Show-StatusReport -Instance $vsInstance
    }
    "doctor" {
        Show-StatusReport -Instance $vsInstance
        Invoke-Cargo -Arguments @(
            "run",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver",
            "--",
            "doctor",
            $workspaceRoot
        )
    }
    "cargo-build-driver" {
        Invoke-Cargo -Arguments @(
            "build",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver"
        )
    }
    "cargo-clean" {
        Invoke-Cargo -Arguments @(
            "clean",
            "--manifest-path",
            $cargoManifestPath
        )
    }
    "cargo-rebuild-driver" {
        Invoke-Cargo -Arguments @(
            "clean",
            "--manifest-path",
            $cargoManifestPath
        )
        Invoke-Cargo -Arguments @(
            "build",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver"
        )
    }
    "cargo-check" {
        Invoke-Cargo -Arguments @(
            "check",
            "--manifest-path",
            $cargoManifestPath
        )
    }
    "cargo-test" {
        Invoke-Cargo -Arguments @(
            "test",
            "--manifest-path",
            $cargoManifestPath
        )
    }
    "cargo-fmt-check" {
        Invoke-Cargo -Arguments @(
            "fmt",
            "--manifest-path",
            $cargoManifestPath,
            "--",
            "--check"
        )
    }
    "build-agam-file" {
        if (-not $Path) {
            throw "-Path is required for the build-agam-file task."
        }

        Invoke-Cargo -Arguments @(
            "run",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver",
            "--",
            "build",
            $Path,
            "--fast"
        )
    }
    "run-agam-file" {
        if (-not $Path) {
            throw "-Path is required for the run-agam-file task."
        }

        Invoke-Cargo -Arguments @(
            "run",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver",
            "--",
            "run",
            $Path,
            "--backend",
            "jit"
        )
    }
    "llvm-smoke" {
        Invoke-Cargo -Arguments @(
            "run",
            "--manifest-path",
            $cargoManifestPath,
            "-p",
            "agam_driver",
            "--",
            "build",
            $llvmSmokePath,
            "--fast"
        )
    }
}
