# tint-icons.ps1
# Generate tinted tray icon variants for VoiceType.
# Uses .NET System.Drawing to overlay color on the base 32x32.png icon.

param(
    [string]$BaseIcon = "src-tauri/icons/32x32.png",
    [string]$OutDir   = "src-tauri/icons"
)

Add-Type -AssemblyName System.Drawing

$colors = @(
    @{ Name = "tray-idle";          R = 0x88; G = 0x88; B = 0x88 }
    @{ Name = "tray-recording";     R = 0xFF; G = 0x33; B = 0x33 }
    @{ Name = "tray-transcribing";  R = 0xFF; G = 0xAA; B = 0x00 }
)

# Resolve paths
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not (Test-Path $BaseIcon)) {
    $BaseIcon = Join-Path $scriptDir $BaseIcon
}
if (-not (Test-Path $BaseIcon)) {
    Write-Error "Base icon not found at: $BaseIcon"
    exit 1
}
if (-not (Test-Path $OutDir)) {
    $OutDir = Join-Path $scriptDir $OutDir
}

Write-Host "Source: $BaseIcon"
Write-Host "Output: $OutDir"

$src = [System.Drawing.Image]::FromFile($BaseIcon)
$w = $src.Width
$h = $src.Height

foreach ($c in $colors) {
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    # Draw original
    $g.DrawImage($src, 0, 0, $w, $h)
    # Tint overlay
    $alpha = 140
    $color = [System.Drawing.Color]::FromArgb($alpha, $c.R, $c.G, $c.B)
    $brush = New-Object System.Drawing.SolidBrush($color)
    $g.FillRectangle($brush, 0, 0, $w, $h)
    $brush.Dispose()
    $g.Dispose()

    $outFile = Join-Path $OutDir "$($c.Name).png"
    $bmp.Save($outFile, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Created: $($c.Name).png"
}

$src.Dispose()
Write-Host "Done - 3 tinted tray icons generated."
