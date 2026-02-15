# Prepare OpenClaw bundle for Tauri resources (Windows)
# Downloads Node.js binary and builds pruned OpenClaw deployment

param(
    [string]$Target = "",
    [string]$NodeVersion = "24.13.1"
)

$ErrorActionPreference = "Stop"
# Ensure native command failures are caught
$PSNativeCommandUseErrorActionPreference = $true

# Check prerequisites
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    Write-Error "Required command 'pnpm' not found"
    exit 1
}

# Paths
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$AppDir = Split-Path -Parent $ScriptDir
$OpenClawDir = if ($env:OPENCLAW_SRC) { $env:OPENCLAW_SRC } else { Join-Path $AppDir "..\openclaw" }
$ResourcesDir = Join-Path $AppDir "src-tauri\resources"

# Auto-detect or validate target
if (-not $Target) {
    $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($Arch) {
        "X64"   { $NodeArch = "x64" }
        "Arm64" { $NodeArch = "arm64" }
        default { Write-Error "Unsupported architecture: $Arch"; exit 1 }
    }
    $Target = "win-$NodeArch"
} else {
    if ($Target -notmatch "^win-(x64|arm64)$") {
        Write-Error "Invalid target: $Target. Expected: win-x64, win-arm64"
        exit 1
    }
    $NodeArch = $Target -replace "^win-", ""
}

# Validate source directories
if (-not (Test-Path $OpenClawDir)) {
    Write-Error "OpenClaw source directory not found at $OpenClawDir"
    exit 1
}

# Node.js download URL
$NodeDist = "node-v${NodeVersion}-win-${NodeArch}"
$NodeUrl = "https://nodejs.org/dist/v${NodeVersion}/${NodeDist}.zip"

Write-Host "=== ClawRunner Bundle Preparation ==="
Write-Host "Target:    $Target"
Write-Host "Node.js:   v$NodeVersion"
Write-Host "OpenClaw:  $OpenClawDir"
Write-Host "Resources: $ResourcesDir"
Write-Host ""

# --- Step 1: Create resources directory ---
New-Item -ItemType Directory -Force -Path (Join-Path $ResourcesDir "openclaw") | Out-Null

# --- Step 2: Download Node.js binary ---
$NodeExe = Join-Path $ResourcesDir "node.exe"
$NodeVersionFile = Join-Path $ResourcesDir ".node-version"
$NodeVersionTag = "v${NodeVersion}-win-${NodeArch}"
$NodeCached = (Test-Path $NodeExe) -and (Test-Path $NodeVersionFile) -and ("$(Get-Content $NodeVersionFile -Raw)".Trim() -eq $NodeVersionTag)
if ($NodeCached) {
    Write-Host ">>> Node.js v${NodeVersion} binary already present, skipping download"
} else {
    if (Test-Path $NodeExe) { Remove-Item -Force $NodeExe }
    if (Test-Path $NodeVersionFile) { Remove-Item -Force $NodeVersionFile }
    Write-Host ">>> Downloading Node.js v${NodeVersion} for ${Target}..."
    $TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "openclaw-node-dl"
    if (Test-Path $TmpDir) { Remove-Item -Recurse -Force $TmpDir }
    New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

    try {
        $ZipPath = Join-Path $TmpDir "node.zip"
        Invoke-WebRequest -Uri $NodeUrl -OutFile $ZipPath -UseBasicParsing
        Expand-Archive -Path $ZipPath -DestinationPath $TmpDir
        Copy-Item (Join-Path $TmpDir "$NodeDist\node.exe") $NodeExe
        Set-Content -Path $NodeVersionFile -Value $NodeVersionTag -NoNewline
        Write-Host "    Node.js binary downloaded"
    } finally {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    }
}

# --- Step 3: Build OpenClaw ---
Write-Host ">>> Building OpenClaw..."
Push-Location $OpenClawDir
try {
    pnpm install --frozen-lockfile
    pnpm build
} finally {
    Pop-Location
}

# --- Step 4: Create pruned production deployment ---
Write-Host ">>> Creating pruned production deployment (pnpm deploy --prod)..."
$DeployDir = Join-Path $AppDir ".openclaw-deploy"
if (Test-Path $DeployDir) { Remove-Item -Recurse -Force $DeployDir }

Push-Location $OpenClawDir
try {
    pnpm --filter openclaw deploy --prod --legacy $DeployDir
} finally {
    Pop-Location
}

# --- Step 5: Copy to resources ---
Write-Host ">>> Copying to resources..."

try {
    # Verify build outputs exist
    foreach ($Required in @(
        (Join-Path $OpenClawDir "openclaw.mjs"),
        (Join-Path $OpenClawDir "dist"),
        (Join-Path $DeployDir "package.json"),
        (Join-Path $DeployDir "node_modules")
    )) {
        if (-not (Test-Path $Required)) {
            throw "Required build output not found: $Required"
        }
    }

    # Entry point
    Copy-Item (Join-Path $OpenClawDir "openclaw.mjs") (Join-Path $ResourcesDir "openclaw\openclaw.mjs")
    Write-Host "    openclaw.mjs copied"

    # package.json (needed for ESM module resolution)
    Copy-Item (Join-Path $DeployDir "package.json") (Join-Path $ResourcesDir "openclaw\package.json")
    Write-Host "    package.json copied"

    # dist/ (tsdown bundle)
    $DistDst = Join-Path $ResourcesDir "openclaw\dist"
    if (Test-Path $DistDst) { Remove-Item -Recurse -Force $DistDst }
    Copy-Item -Recurse (Join-Path $OpenClawDir "dist") $DistDst
    Write-Host "    dist copied"

    # node_modules/ (pruned production deps with native addons)
    $NodeModulesDst = Join-Path $ResourcesDir "openclaw\node_modules"
    if (Test-Path $NodeModulesDst) { Remove-Item -Recurse -Force $NodeModulesDst }
    Copy-Item -Recurse (Join-Path $DeployDir "node_modules") $NodeModulesDst
    Write-Host "    node_modules copied"
} finally {
    # Cleanup deploy dir
    Remove-Item -Recurse -Force $DeployDir -ErrorAction SilentlyContinue
}

# --- Summary ---
Write-Host ""
Write-Host "=== Bundle Preparation Complete ==="
Write-Host "Resources ready at: $ResourcesDir"
