#Requires -Version 5.1
<#
.SYNOPSIS
    Idempotent dev-env bootstrap for building SnipTeX on Windows 11 (ARM64 or x64).

.DESCRIPTION
    Checks every prerequisite, installs only what's missing, then verifies the
    final toolchain. Safe to re-run. Designed for Parallels VMs on Apple
    Silicon Macs (Windows 11 ARM64) but also works on x64 Windows.

.NOTES
    Run from an admin PowerShell after installing git + cloning the repo:
      winget install Git.Git
      git clone https://github.com/hiep1987/sniptex.git
      cd sniptex
      powershell -ExecutionPolicy Bypass -File scripts\windows-bootstrap.ps1
#>

$ErrorActionPreference = 'Stop'
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq 'Arm64') { 'ARM64' } else { 'X64' }
$rustTarget = if ($arch -eq 'ARM64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }

function Test-Cmd { param([string]$Name) $null -ne (Get-Command $Name -ErrorAction SilentlyContinue) }
function Step { param([string]$Label) Write-Host ""; Write-Host "==> $Label" -ForegroundColor Cyan }
function Ok   { param([string]$Msg)   Write-Host "    [OK]   $Msg" -ForegroundColor Green }
function Skip { param([string]$Msg)   Write-Host "    [SKIP] $Msg" -ForegroundColor Yellow }
function Warn { param([string]$Msg)   Write-Host "    [WARN] $Msg" -ForegroundColor Yellow }

function Install-Winget {
    param([Parameter(Mandatory)][string]$Id, [string]$DisplayName = $Id, [string]$Override)
    Write-Host "    installing $DisplayName via winget ($Id)..." -ForegroundColor White
    $args = @('install', '--id', $Id, '--accept-source-agreements', '--accept-package-agreements', '--silent')
    if ($Override) { $args += @('--override', $Override) }
    & winget @args
    # winget returns 0 on success, -1978335189 (0x8A15002B) when already installed; treat both as OK
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne -1978335189) {
        throw "winget install failed for $Id (exit $LASTEXITCODE)"
    }
    Ok "$DisplayName installed (or already present)"
}

Write-Host "SnipTeX Windows bootstrap" -ForegroundColor Magenta
Write-Host "  arch:         $arch"
Write-Host "  rust target:  $rustTarget"

# ---- 1. winget itself ----
Step "Verify winget"
if (-not (Test-Cmd winget)) {
    throw "winget not found. Install 'App Installer' from Microsoft Store, then re-run."
}
Ok "winget present"

# ---- 2. Git ----
Step "Git"
if (Test-Cmd git) { Ok (git --version) } else { Install-Winget -Id 'Git.Git' -DisplayName 'Git' }

# ---- 3. Rust (rustup + cargo) ----
Step "Rust (rustup + cargo)"
if (Test-Cmd cargo) {
    Ok (cargo --version)
} else {
    Install-Winget -Id 'Rustlang.Rustup' -DisplayName 'Rustup'
    Warn "open a fresh PowerShell after this script so PATH picks up cargo."
}

# ---- 4. Node.js LTS (>= 20) ----
Step "Node.js (LTS, >= 20)"
$nodeNeedsInstall = $true
if (Test-Cmd node) {
    $nodeVer = (node --version).TrimStart('v')
    $major = [int]($nodeVer.Split('.')[0])
    if ($major -ge 20) { Ok "node v$nodeVer"; $nodeNeedsInstall = $false }
    else { Warn "node v$nodeVer < 20; upgrading." }
}
if ($nodeNeedsInstall) { Install-Winget -Id 'OpenJS.NodeJS.LTS' -DisplayName 'Node.js LTS' }

# ---- 5. pnpm via corepack (Node ships corepack) ----
Step "pnpm (via corepack)"
if (Test-Cmd pnpm) {
    Ok ("pnpm " + (pnpm --version))
} elseif (Test-Cmd corepack) {
    corepack enable
    corepack prepare pnpm@latest --activate
    Ok "pnpm activated via corepack"
} else {
    Install-Winget -Id 'pnpm.pnpm' -DisplayName 'pnpm'
}

# ---- 6. Visual Studio 2022 Build Tools (MSVC + Win11 SDK + target-arch VC tools) ----
Step "Visual Studio 2022 Build Tools (MSVC + Win11 SDK)"
$vsBuildToolsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
if (Test-Path $vsBuildToolsPath) {
    Ok "VS 2022 Build Tools detected at $vsBuildToolsPath"
} else {
    $vcComponent = if ($arch -eq 'ARM64') { 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' } else { 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64' }
    $override = "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --add $vcComponent --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --includeRecommended"
    Warn "VS Build Tools install is ~5GB and may take 10-20 min."
    Install-Winget -Id 'Microsoft.VisualStudio.2022.BuildTools' -DisplayName 'VS 2022 Build Tools' -Override $override
}

# ---- 6b. LLVM/Clang (required by the `ring` crate's perlasm build on Windows,
#         especially aarch64-pc-windows-msvc; VS Build Tools alone is not enough) ----
Step "LLVM / Clang (required by ring crate)"
if (Test-Cmd clang) {
    Ok (clang --version | Select-Object -First 1)
} else {
    Install-Winget -Id 'LLVM.LLVM' -DisplayName 'LLVM (clang)'
    Warn "open a fresh PowerShell so PATH picks up C:\Program Files\LLVM\bin."
}

# ---- 7. WebView2 runtime ----
Step "WebView2 runtime"
$wv2Key = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (Test-Path $wv2Key) {
    Ok "WebView2 runtime present (bundled with Edge on Win11)"
} else {
    Install-Winget -Id 'Microsoft.EdgeWebView2Runtime' -DisplayName 'WebView2 Runtime'
}

# ---- 8. VC++ Redistributable for target arch ----
Step "Visual C++ Redistributable"
$vcRedistId = if ($arch -eq 'ARM64') { 'Microsoft.VCRedist.2015+.arm64' } else { 'Microsoft.VCRedist.2015+.x64' }
Install-Winget -Id $vcRedistId -DisplayName "VC++ Redist ($arch)"

# ---- 9. Rust target for native build ----
Step "Rust target: $rustTarget"
if (Test-Cmd rustup) {
    rustup target add $rustTarget
    Ok "$rustTarget installed"
} else {
    Skip "rustup not on PATH yet. Open a fresh PowerShell and run: rustup target add $rustTarget"
}

# ---- 10. Final verification ----
Step "Verify final toolchain"
$tools = [ordered]@{ 'git' = 'git --version'; 'rustc' = 'rustc --version'; 'cargo' = 'cargo --version'; 'clang' = 'clang --version | Select-Object -First 1'; 'node' = 'node --version'; 'pnpm' = 'pnpm --version' }
$missing = @()
foreach ($k in $tools.Keys) {
    if (Test-Cmd $k) {
        $v = Invoke-Expression $tools[$k]
        Write-Host ("    {0,-7} {1}" -f $k, $v) -ForegroundColor Green
    } else {
        Write-Host ("    {0,-7} MISSING" -f $k) -ForegroundColor Red
        $missing += $k
    }
}

Write-Host ""
if ($missing.Count -gt 0) {
    Warn "Some tools still missing: $($missing -join ', '). Open a fresh PowerShell and re-run this script."
} else {
    Write-Host "==> Bootstrap done." -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor White
    Write-Host "  1. Open a fresh PowerShell (so PATH changes apply)."
    Write-Host "  2. cd sniptex"
    Write-Host "  3. pnpm install"
    Write-Host "  4. pnpm tauri build --bundles msi"
    Write-Host "  5. Walk the Batch B checklist:"
    Write-Host "       plans\260520-0603-sniptex-tauri-mvp-v1\phase-10-windows-cross-platform-port.md"
}
