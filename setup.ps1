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

# Copy personal dictionary template if not already present
$dictDest = "$PSScriptRoot\src\sidecar\personal_dictionary.json"
$dictSrc  = "$PSScriptRoot\src\sidecar\personal_dictionary.json.example"
if (-not (Test-Path $dictDest)) {
    Copy-Item $dictSrc $dictDest
    Write-Host "Created personal_dictionary.json from template."
}

Write-Host ""
Write-Host "Setup complete. Next steps:"
Write-Host "  1. cd src\sidecar && pip install -r requirements.txt"
Write-Host "  2. cd src\shell   && cargo build --release"
Write-Host "  3. Run: src\shell\target\release\inkvoice-shell.exe"
