# Prepare OpenClaw bundle for Tauri resources (Windows)
# Downloads Node.js binary and copies OpenClaw dist + node_modules

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$AppDir = Split-Path -Parent $ScriptDir
$ResourcesDir = Join-Path $AppDir "src-tauri\resources"
$OpenClawSrc = if ($env:OPENCLAW_SRC) { $env:OPENCLAW_SRC } else { Join-Path $AppDir "..\openclaw" }

$NodeVersion = "v22.15.0"
$NodeArch = "x64"
$NodeFilename = "node-${NodeVersion}-win-${NodeArch}"
$NodeUrl = "https://nodejs.org/dist/${NodeVersion}/${NodeFilename}.zip"

Write-Host "=== OpenClaw Desktop Bundle Preparation ==="
Write-Host "Platform: win-${NodeArch}"
Write-Host "Node.js: ${NodeVersion}"
Write-Host "OpenClaw source: ${OpenClawSrc}"
Write-Host ""

# Create resources directory
New-Item -ItemType Directory -Force -Path "$ResourcesDir\openclaw" | Out-Null

# Download Node.js binary if not present
$NodeExe = Join-Path $ResourcesDir "node.exe"
if (-not (Test-Path $NodeExe)) {
    Write-Host ">>> Downloading Node.js ${NodeVersion} for win-${NodeArch}..."
    $TmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $ZipPath = Join-Path $TmpDir.FullName "node.zip"
    Invoke-WebRequest -Uri $NodeUrl -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir.FullName
    Copy-Item (Join-Path $TmpDir.FullName "${NodeFilename}\node.exe") $NodeExe
    Remove-Item -Recurse -Force $TmpDir.FullName
    Write-Host "    Node.js binary downloaded"
} else {
    Write-Host ">>> Node.js binary already present, skipping download"
}

# Copy OpenClaw dist
$DistSrc = Join-Path $OpenClawSrc "dist"
if (Test-Path $DistSrc) {
    Write-Host ">>> Copying OpenClaw dist..."
    $DistDst = Join-Path $ResourcesDir "openclaw\dist"
    if (Test-Path $DistDst) { Remove-Item -Recurse -Force $DistDst }
    Copy-Item -Recurse $DistSrc $DistDst
} else {
    Write-Host "WARNING: OpenClaw dist not found at $DistSrc"
}

# Copy openclaw.mjs
$EntryPoint = Join-Path $OpenClawSrc "openclaw.mjs"
if (Test-Path $EntryPoint) {
    Write-Host ">>> Copying openclaw.mjs..."
    Copy-Item $EntryPoint (Join-Path $ResourcesDir "openclaw\openclaw.mjs")
} else {
    Write-Host "WARNING: openclaw.mjs not found at $EntryPoint"
}

# Copy node_modules
$NodeModulesSrc = Join-Path $OpenClawSrc "node_modules"
if (Test-Path $NodeModulesSrc) {
    Write-Host ">>> Copying node_modules..."
    $NodeModulesDst = Join-Path $ResourcesDir "openclaw\node_modules"
    if (Test-Path $NodeModulesDst) { Remove-Item -Recurse -Force $NodeModulesDst }
    Copy-Item -Recurse $NodeModulesSrc $NodeModulesDst
} else {
    Write-Host "WARNING: node_modules not found at $NodeModulesSrc"
}

# Copy package.json
$PkgJson = Join-Path $OpenClawSrc "package.json"
if (Test-Path $PkgJson) {
    Copy-Item $PkgJson (Join-Path $ResourcesDir "openclaw\package.json")
}

Write-Host ""
Write-Host "=== Bundle Preparation Complete ==="
