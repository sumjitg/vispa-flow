# build-python-sidecar.ps1
# Build the Python sidecar into a standalone .exe using PyInstaller.
# Output: src-tauri/binaries/voicetype-python-x86_64-pc-windows-msvc.exe
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts/build-python-sidecar.ps1

param(
    [string]$PythonExe = "python",
    [string]$SpecFile = "python/voicetype-python.spec",
    [string]$OutDir   = "src-tauri/binaries"
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectDir = Split-Path -Parent $scriptDir

Push-Location $projectDir

# Ensure PyInstaller is installed
try {
    & $PythonExe -c "import PyInstaller" 2>&1>$null
} catch {
    Write-Host "Installing PyInstaller..."
    & $PythonExe -m pip install pyinstaller --quiet 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to install PyInstaller"
        Pop-Location
        exit 1
    }
}

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path (Join-Path $projectDir $OutDir) | Out-Null

# Build with PyInstaller
Write-Host "Building Python sidecar with PyInstaller..."
& $PythonExe -m PyInstaller $SpecFile --distpath $OutDir --workpath build/pyinstaller --clean --noconfirm 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "PyInstaller build failed"
    Pop-Location
    exit 1
}

# Rename output to match Tauri's expected sidecar naming convention
# Tauri expects: <name>-<target_triple>.exe
$targetTriple = "x86_64-pc-windows-msvc"
$builtExe = Join-Path $outDir "voicetype-python.exe"
$expectedExe = Join-Path $outDir "voicetype-python-${targetTriple}.exe"

if (Test-Path $expectedExe) {
    Remove-Item $expectedExe -Force
}
if (Test-Path $builtExe) {
    Rename-Item $builtExe "voicetype-python-${targetTriple}.exe"
    Write-Host "Created: $expectedExe"
} else {
    Write-Error "Built exe not found at: $builtExe"
    Pop-Location
    exit 1
}

Pop-Location
Write-Host "Python sidecar build complete."
Write-Host "Run 'npm run tauri build' to produce the MSI installer."
