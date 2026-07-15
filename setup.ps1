# InkVoice setup script
# Downloads the Parakeet ASR model from the GitHub release and places it
# where asr_server.py expects it.
#
# Prerequisites: gh CLI authenticated, or just download manually from:
# https://github.com/Prab81/inkvoice/releases/tag/v0.1.0

$modelDir = "$PSScriptRoot\spikes\m0_asr\sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"

if (Test-Path "$modelDir\encoder.int8.onnx") {
    Write-Host "Model already present at $modelDir — skipping download."
} else {
    Write-Host "Downloading Parakeet model from GitHub release..."
    New-Item -ItemType Directory -Force -Path $modelDir | Out-Null

    gh release download v0.1.0 `
        --repo Prab81/inkvoice `
        --pattern "*.onnx" `
        --pattern "tokens.txt" `
        --dir $modelDir

    Write-Host "Model downloaded to $modelDir"
}

# Download pre-built sidecar exe (no Python/pip needed)
$sidecarExe = "$PSScriptRoot\inkvoice-sidecar.exe"
if (-not (Test-Path $sidecarExe)) {
    Write-Host "Downloading inkvoice-sidecar.exe from GitHub release..."
    gh release download v0.1.0 `
        --repo Prab81/inkvoice `
        --pattern "inkvoice-sidecar.exe" `
        --dir $PSScriptRoot
    Write-Host "inkvoice-sidecar.exe downloaded."
} else {
    Write-Host "inkvoice-sidecar.exe already present — skipping."
}

# Copy personal dictionary template if not already present
$dictDest = "$PSScriptRoot\src\sidecar\personal_dictionary.json"
$dictSrc  = "$PSScriptRoot\src\sidecar\personal_dictionary.json.example"
if (-not (Test-Path $dictDest)) {
    Copy-Item $dictSrc $dictDest
    Write-Host "Created personal_dictionary.json from template."
}

Write-Host ""
Write-Host "Setup complete. To run InkVoice:"
Write-Host "  1. Start sidecar:  .\inkvoice-sidecar.exe"
Write-Host "  2. Start shell:    src\shell\target\release\inkvoice-shell.exe"
Write-Host ""
Write-Host "  (Or build the shell from source: cd src\shell && cargo build --release)"
