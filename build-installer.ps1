# build-installer.ps1
# Build the Windows installer for qtz-handy (Tauri) and open it.
# Run this from an **elevated** PowerShell (Run as Administrator).

[CmdletBinding()]
param(
    [switch]$SkipInstall,
    [string]$LibClangPath
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $projectRoot -or $projectRoot -eq "") { $projectRoot = (Get-Location).Path }

Write-Host "=== qtz-handy Installer Builder ===" -ForegroundColor Cyan
Write-Host "Project: $projectRoot"
Write-Host ""

# 1. Admin check
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning "This script should be run from an elevated PowerShell (Run as Administrator)."
    Write-Host "Right-click PowerShell -> Run as Administrator, then cd to the folder and run: .\build-installer.ps1"
    # Try to self-elevate (will prompt UAC)
    $args = "-NoProfile -ExecutionPolicy Bypass -File `"$($MyInvocation.MyCommand.Path)`" " + ($MyInvocation.BoundParameters.GetEnumerator() | ForEach-Object { if ($_.Value -is [switch]) { "-$($_.Key)" } else { "-$($_.Key) `"$($_.Value)`"" } }) -join " "
    Start-Process powershell -Verb RunAs -ArgumentList $args
    exit
}

# 2. Ensure we are in the project
Push-Location $projectRoot

# Ensure bun is installed and in PATH (install if missing)
if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
    Write-Host "Bun not found in PATH. Installing Bun..." -ForegroundColor Yellow
    try {
        $installCmd = 'irm bun.sh/install.ps1 | iex'
        $installOutput = powershell -NoProfile -ExecutionPolicy Bypass -Command $installCmd 2>&1 | Out-String
        Write-Host $installOutput
        # Add to current session PATH
        $bunBin = Join-Path $env:USERPROFILE ".bun\bin"
        if (Test-Path $bunBin) {
            $env:Path = "$bunBin;$env:Path"
            Write-Host "Added $bunBin to PATH for this session." -ForegroundColor Green
        }
        if (Get-Command bun -ErrorAction SilentlyContinue) {
            Write-Host "Bun installed successfully." -ForegroundColor Green
        } else {
            Write-Warning "Bun installed but PATH may need refresh. Restart PowerShell if 'bun' not found."
        }
    } catch {
        Write-Error "Failed to install Bun automatically. Please install manually: https://bun.sh/ (run in terminal: powershell -c `"irm bun.sh/install.ps1|iex`") then restart this session."
        exit 1
    }
}

# Use short CARGO_TARGET_DIR to avoid Windows MAX_PATH (260 char) issues with deep paths in whisper-rs-sys cmake builds
$global:ShortCargoTarget = "D:\b"
if (-not (Test-Path $global:ShortCargoTarget)) {
    New-Item -ItemType Directory -Path $global:ShortCargoTarget -Force | Out-Null
}
$env:CARGO_TARGET_DIR = $global:ShortCargoTarget
Write-Host "Using short CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR to prevent path length errors." -ForegroundColor DarkGray

# 3. Install LLVM if needed (provides libclang for whisper-rs-sys bindgen)
function Ensure-LibClang {
    if ($LibClangPath -and (Test-Path $LibClangPath)) {
        $env:LIBCLANG_PATH = $LibClangPath
        Write-Host "Using provided LIBCLANG_PATH: $env:LIBCLANG_PATH" -ForegroundColor Green
        return $true
    }

    $candidates = @(
        "C:\ProgramData\chocolatey\lib\llvm\tools\LLVM\bin",
        "C:\Program Files\LLVM\bin",
        "C:\Program Files (x86)\LLVM\bin",
        "D:\Program Files\LLVM\bin"
    )

    foreach ($c in $candidates) {
        $dll = Join-Path $c "libclang.dll"
        if (Test-Path $dll) {
            $env:LIBCLANG_PATH = $c
            Write-Host "Found libclang at: $env:LIBCLANG_PATH" -ForegroundColor Green
            return $true
        }
    }

    if ($SkipInstall) {
        Write-Warning "No libclang found and -SkipInstall was used. Set LIBCLANG_PATH manually."
        return $false
    }

    Write-Host "LLVM / libclang not found. Installing via winget or choco (this may take a few minutes)..." -ForegroundColor Yellow

    $installed = $false
    try {
        winget install --id LLVM.LLVM -e --accept-package-agreements --accept-source-agreements --silent | Out-Null
        $installed = $true
    } catch {
        Write-Host "winget install attempt finished (may already be present or require manual steps)." -ForegroundColor DarkGray
    }

    if (-not $installed -or -not (Test-Path "C:\Program Files\LLVM\bin\libclang.dll")) {
        try {
            choco install llvm -y --no-progress | Out-Null
            $installed = $true
        } catch {
            Write-Warning "choco also had issues: $_"
        }
    }

    # Re-check candidates after install
    Start-Sleep -Seconds 2
    foreach ($c in $candidates) {
        $dll = Join-Path $c "libclang.dll"
        if (Test-Path $dll) {
            $env:LIBCLANG_PATH = $c
            Write-Host "LLVM installed. LIBCLANG_PATH set to: $env:LIBCLANG_PATH" -ForegroundColor Green
            return $true
        }
    }

    # Final guess for winget default
    $possible = "C:\Program Files\LLVM\bin"
    if (Test-Path (Join-Path $possible "libclang.dll")) {
        $env:LIBCLANG_PATH = $possible
        return $true
    }

    Write-Error "Failed to locate libclang.dll after install attempts. Please install LLVM manually and set `$env:LIBCLANG_PATH` to its bin folder, then re-run."
    return $false
}

# 4. Setup MSVC environment (vcvars64)
function Invoke-VcVars {
    $vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
    if (-not (Test-Path $vcvars)) {
        $vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
    }
    if (-not (Test-Path $vcvars)) {
        Write-Warning "vcvars64.bat not found in expected locations. Assuming MSVC env is already in PATH (cl.exe available)."
        $global:VcVarsBat = $null
        return
    }

    Write-Host "Calling vcvars64 to configure MSVC compiler environment..." -ForegroundColor DarkCyan
    $global:VcVarsBat = $vcvars
}

# 5. Run the build
function Build-Installer {
    Write-Host "Running bun install (if needed)..." -ForegroundColor DarkGray
    $oldPref = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = bun install --frozen-lockfile 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $oldPref
    if ($exitCode -ne 0) {
        Write-Host $output -ForegroundColor Red
        if (-not (Test-Path "node_modules")) {
            Write-Error "bun install failed with exit code $exitCode and no node_modules was created. Run 'bun install --frozen-lockfile' manually to see the full error."
            exit $exitCode
        }
        Write-Warning "bun install reported non-zero ($exitCode) but node_modules exists. Continuing (common if postinstall scripts fail on Windows)."
    }

    Write-Host "Building with Tauri (this will take several minutes on first release build)..." -ForegroundColor Cyan

    # Ensure short target for this build
    if ($global:ShortCargoTarget) {
        $env:CARGO_TARGET_DIR = $global:ShortCargoTarget
    }

    if ($global:VcVarsBat -and (Test-Path $global:VcVarsBat)) {
        $cmd = '"{0}" >nul && cd /d "{1}" && bun run tauri build' -f $global:VcVarsBat, $projectRoot
        & cmd /c $cmd
    } else {
        Write-Warning "vcvars not found, running build without it (may fail if MSVC not in PATH)"
        & bun run tauri build
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Tauri build failed with exit code $LASTEXITCODE. Check output above."
        exit $LASTEXITCODE
    }
}

# 6. Find and open the installer
function Open-Installer {
    # Use short target if set (for consistency with build)
    $cargoTarget = if ($global:ShortCargoTarget -and (Test-Path $global:ShortCargoTarget)) {
        $global:ShortCargoTarget
    } elseif ($env:CARGO_TARGET_DIR -and (Test-Path $env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR
    } else {
        Join-Path $projectRoot "src-tauri\target"
    }

    $bundleRoot = if ($cargoTarget) {
        Join-Path $cargoTarget "release\bundle"
    } else {
        Join-Path $projectRoot "src-tauri\target\release\bundle"
    }

    if (-not (Test-Path $bundleRoot)) {
        Write-Error "Bundle directory not found after build: $bundleRoot"
        exit 1
    }

    $msi = Get-ChildItem $bundleRoot -Recurse -Filter "*.msi" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $nsisSetup = Get-ChildItem $bundleRoot -Recurse -Filter "*setup*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $exe = Get-ChildItem $bundleRoot -Recurse -Filter "*.exe" | Where-Object { $_.Name -notlike "*setup*" -and $_.Name -notlike "*portable*" } | Sort-Object LastWriteTime -Descending | Select-Object -First 1

    if ($msi) {
        Write-Host "Found installer: $($msi.FullName)" -ForegroundColor Green
        Write-Host "Opening installer..." -ForegroundColor Cyan
        Invoke-Item $msi.FullName
        Write-Host "Installer launched. Complete the setup wizard to install Handy." -ForegroundColor Green
        return $msi.FullName
    } elseif ($nsisSetup) {
        Write-Host "Found installer: $($nsisSetup.FullName)" -ForegroundColor Green
        Write-Host "Opening installer..." -ForegroundColor Cyan
        Invoke-Item $nsisSetup.FullName
        Write-Host "Installer launched. Complete the setup wizard to install Handy." -ForegroundColor Green
        return $nsisSetup.FullName
    } elseif ($exe) {
        Write-Host "Found installer: $($exe.FullName)" -ForegroundColor Green
        Write-Host "Opening installer..." -ForegroundColor Cyan
        Invoke-Item $exe.FullName
        return $exe.FullName
    } else {
        Write-Host "Build succeeded but no .msi/.exe found in $bundleRoot" -ForegroundColor Yellow
        Write-Host "Contents:"
        Get-ChildItem $bundleRoot -Recurse | Select FullName
        return $null
    }
}

# === Main flow ===
if (-not (Ensure-LibClang)) {
    Write-Error "Cannot proceed without libclang. Install LLVM and retry."
    exit 1
}

Invoke-VcVars
Build-Installer
$installedPath = Open-Installer

Write-Host ""
Write-Host "=== Done ===" -ForegroundColor Green
if ($installedPath) {
    Write-Host "Installer opened: $installedPath"
}
$finalExe = if ($global:ShortCargoTarget) { Join-Path $global:ShortCargoTarget "release\handy.exe" } else { "src-tauri\target\release\handy.exe" }
Write-Host "You can also run the built app directly from: $finalExe (after install or with resources)."

Pop-Location
