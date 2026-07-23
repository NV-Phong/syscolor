# build-release.ps1
# Builds the release binary and MSI installer for syscolor.
# Requires: Rust toolchain, cargo-wix, WiX Toolset v3.14

[CmdletBinding()]
param(
    [string]$WixBinPath = "C:\Program Files (x86)\WiX Toolset v3.14\bin"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Ensure WiX is on PATH ──────────────────────────────────────────────────────
if (Test-Path $WixBinPath) {
    $env:PATH = "$WixBinPath;$env:PATH"
    Write-Host "[+] WiX bin added: $WixBinPath" -ForegroundColor Cyan
} else {
    Write-Warning "WiX not found at '$WixBinPath'. Make sure WiX Toolset v3.14 is installed."
    Write-Warning "Download: https://github.com/wixtoolset/wix3/releases/tag/wix3141rtm"
    exit 1
}

# ── Read version from Cargo.toml ───────────────────────────────────────────────
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $version = $Matches[1]
} else {
    Write-Error "Could not read version from Cargo.toml"
    exit 1
}
Write-Host "[+] Building v$version" -ForegroundColor Cyan

# ── Build release binary ───────────────────────────────────────────────────────
Write-Host "[+] cargo build --release" -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# ── Build MSI ─────────────────────────────────────────────────────────────────
Write-Host "[+] cargo wix" -ForegroundColor Cyan
cargo wix --nocapture
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# ── Report output ─────────────────────────────────────────────────────────────
$msi = Get-Item "target\wix\*.msi" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($msi) {
    $sizeKb = [math]::Round($msi.Length / 1KB, 1)
    Write-Host ""
    Write-Host "✓ MSI ready: $($msi.FullName)  ($sizeKb KB)" -ForegroundColor Green
} else {
    Write-Error "MSI not found under target\wix\"
}
