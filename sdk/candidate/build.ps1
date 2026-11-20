$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    $ModsDirectory = Split-Path $PSScriptRoot -Parent
    $SiblingSdk = Join-Path $ModsDirectory "mod-api-stable"
    $SdkCandidates = @()

    if ($env:TFM2_STABLE_SDK_DIR) {
        $SdkCandidates += (Join-Path $env:TFM2_STABLE_SDK_DIR "mod-api-stable")
        $SdkCandidates += $env:TFM2_STABLE_SDK_DIR
    }

    $GameRoot = Split-Path $ModsDirectory -Parent
    $SdkCandidates += (Join-Path $GameRoot "mod-sdk-stable\mod-api-stable")

    if (!(Test-Path $SiblingSdk)) {
        $BundledSdk = $SdkCandidates | Where-Object { Test-Path (Join-Path $_ "Cargo.toml") } | Select-Object -First 1
        if ($BundledSdk) {
            New-Item -ItemType Junction -Path $SiblingSdk -Target $BundledSdk | Out-Null
        }
    }

    if (!(Test-Path (Join-Path $SiblingSdk "Cargo.toml"))) {
        throw "Stable SDK not found. Put mod-api-stable next to this mod under mods/, or set TFM2_STABLE_SDK_DIR."
    }

    cargo build --release
    Copy-Item ".\target\release\mlbb_heroes.dll" ".\mlbb_heroes.dll" -Force
    Write-Host "Built .\mlbb_heroes.dll"
} finally {
    Pop-Location
}
