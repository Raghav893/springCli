# springup installer for Windows — https://github.com/Raghav893/springCli
# Usage: irm https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.ps1 | iex
#
# Downloads the latest springup binary for Windows, installs it to
# ~/.springup/bin, and adds that directory to your PATH.

$ErrorActionPreference = "Stop"

$Repo = "Raghav893/springCli"
$BinaryName = "springup.exe"
$InstallDir = "$env:USERPROFILE\.springup\bin"

# ── Helpers ─────────────────────────────────────────────────────────
function Write-Info    { param($Msg) Write-Host "  info  " -ForegroundColor Cyan -NoNewline; Write-Host " $Msg" }
function Write-Success { param($Msg) Write-Host "  ✔     " -ForegroundColor Green -NoNewline; Write-Host " $Msg" }
function Write-Err     { param($Msg) Write-Host "  error " -ForegroundColor Red -NoNewline; Write-Host " $Msg"; exit 1 }

# ── Get latest version from GitHub ──────────────────────────────────
function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "springup-installer" }
        return $release.tag_name
    } catch {
        Write-Err "Could not fetch latest release. Check https://github.com/$Repo/releases"
    }
}

# ── Download & Install ──────────────────────────────────────────────
function Install-Springup {
    Write-Host ""
    Write-Host "  springup installer" -ForegroundColor Cyan -NoNewline
    Write-Host ""
    Write-Host "  Scaffold production-ready Spring Boot projects in seconds." -ForegroundColor Cyan
    Write-Host ""

    # Detect architecture
    $Arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { Write-Err "springup requires a 64-bit system." }
    $Target = "x86_64-pc-windows-msvc"

    Write-Info "Detected platform: Windows $Arch → $Target"

    # Get version
    $Version = Get-LatestVersion
    Write-Info "Latest version: $Version"

    # Download
    $AssetName = "springup-${Target}.zip"
    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$AssetName"
    Write-Info "Downloading springup $Version..."

    $TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "springup-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

    $ZipPath = Join-Path $TmpDir "springup.zip"

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
    } catch {
        # Try raw binary download (no .zip extension)
        $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/springup-${Target}.exe"
        try {
            Invoke-WebRequest -Uri $DownloadUrl -OutFile (Join-Path $TmpDir $BinaryName) -UseBasicParsing
        } catch {
            Write-Err "Download failed. The release asset may not exist yet.`n  Check: https://github.com/$Repo/releases/tag/$Version"
        }
    }

    # Extract if zip was downloaded
    if (Test-Path $ZipPath) {
        Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

        # Find the binary
        $BinaryPath = Get-ChildItem -Path $TmpDir -Filter $BinaryName -Recurse | Select-Object -First 1 -ExpandProperty FullName
        if (-not $BinaryPath) {
            Write-Err "Could not find $BinaryName in the downloaded archive."
        }
    } else {
        $BinaryPath = Join-Path $TmpDir $BinaryName
    }

    # Install
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    Copy-Item -Path $BinaryPath -Destination (Join-Path $InstallDir $BinaryName) -Force
    Write-Success "Installed springup to $InstallDir\$BinaryName"

    # Clean up
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue

    # Add to PATH
    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
        $env:Path = "$InstallDir;$env:Path"
        Write-Success "Added $InstallDir to your PATH"
    } else {
        Write-Success "$InstallDir is already in your PATH"
    }

    Write-Host ""
    Write-Host "  ✔ springup $Version installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Next steps:" -ForegroundColor White
    Write-Host ""
    Write-Host "    1. " -ForegroundColor Yellow -NoNewline; Write-Host "Restart your terminal"
    Write-Host "    2. " -ForegroundColor Yellow -NoNewline; Write-Host "Verify:  " -NoNewline; Write-Host "springup --version" -ForegroundColor Cyan
    Write-Host "    3. " -ForegroundColor Yellow -NoNewline; Write-Host "Create:  " -NoNewline; Write-Host "springup new" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  Documentation: https://github.com/$Repo" -ForegroundColor White
    Write-Host ""
}

Install-Springup
