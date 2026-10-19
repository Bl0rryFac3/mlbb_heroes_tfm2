$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    cargo build --release
    Copy-Item ".\target\release\mlbb_heroes.dll" ".\mlbb_heroes.dll" -Force
    Write-Host "Built .\mlbb_heroes.dll"
} finally {
    Pop-Location
}
