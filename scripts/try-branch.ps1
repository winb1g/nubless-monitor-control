# Runs a branch of nubless on Windows with one command: installs what is missing
# (Git, Node, Rust, the C++ build tools), fetches the branch, and starts the app.
#
#   irm https://raw.githubusercontent.com/winb1g/nubless-monitor-control/claude/admiring-sagan-1dh6ll/scripts/try-branch.ps1 | iex
#
# or save it and run:  powershell -ExecutionPolicy Bypass -File try-branch.ps1
# To try another branch, change $Branch below.
#
# Safe to run again: it updates the checkout and starts the app. The first run
# takes 10-20 minutes (mostly compiling); later runs start in under a minute.
# Nothing here writes to your monitor; the app does that only when you use it.

$Branch = "claude/admiring-sagan-1dh6ll"
$Dir = Join-Path $HOME "nubless-test"
$ErrorActionPreference = "Stop"
$Repo = "https://github.com/winb1g/nubless-monitor-control.git"

function Say($text) { Write-Host "`n==> $text" -ForegroundColor Cyan }
function Has($cmd) { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

# Installs change PATH for new shells only; pull the new PATH into this one.
function Refresh-Path {
  $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $user = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = "$machine;$user;$HOME\.cargo\bin"
}

function Install($id, $what, [string]$extra = "") {
  if (-not (Has winget)) {
    throw "winget is missing, so I can't install $what for you. Install it by hand, then run this again."
  }
  Say "Installing $what (a Windows prompt may ask for permission)"
  $wingetArgs = @("install", "--id", $id, "-e", "--accept-source-agreements", "--accept-package-agreements")
  if ($extra) { $wingetArgs += @("--override", $extra) }
  & winget @wingetArgs
  Refresh-Path
}

Say "Checking what you already have"
Refresh-Path

if (-not (Has git)) { Install "Git.Git" "Git" }

if (-not (Has node)) { Install "OpenJS.NodeJS.LTS" "Node.js" }

# Rust on Windows needs Microsoft's C++ build tools to link.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasCpp = (Test-Path $vswhere) -and
  (& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if (-not $hasCpp) {
  Install "Microsoft.VisualStudio.2022.BuildTools" "the C++ build tools (a large download, about 3 GB)" `
    "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
}

if (-not (Has cargo)) {
  Install "Rustlang.Rustup" "Rust"
  if (Has rustup) { & rustup default stable }
}

foreach ($tool in @("git", "node", "npm", "cargo")) {
  if (-not (Has $tool)) {
    throw "'$tool' still isn't available. Close this window, open a new PowerShell and run the command again."
  }
}

Say "Getting the branch ($Branch) into $Dir"
if (Test-Path (Join-Path $Dir ".git")) {
  git -C $Dir fetch origin $Branch
  git -C $Dir checkout -B $Branch FETCH_HEAD
} else {
  git clone --branch $Branch --single-branch $Repo $Dir
}
Write-Host "At: $(git -C $Dir log --oneline -1)"

if (Get-Process | Where-Object { $_.ProcessName -like "LG*Switch*" -or $_.ProcessName -like "SmartMonitorControl*" }) {
  Write-Host "`nLG Switch looks like it is running. Quit it first (tray icon -> Exit): its service also talks to the monitor." -ForegroundColor Yellow
  Read-Host "Press Enter when it is closed"
}

Say "Installing the app's web dependencies"
Push-Location (Join-Path $Dir "apps\desktop")
npm ci

Say "Starting nubless. The first build compiles Rust and takes a while; a window opens when it's ready."
Write-Host "Then: Settings -> turn on Labs -> open Displays -> follow docs\hardware-test-cards.md`n"
npx tauri dev
Pop-Location
