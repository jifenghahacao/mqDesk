param(
    [Parameter(Mandatory=$true)]
    [string]$Url,
    [Parameter(Mandatory=$true)]
    [string]$OutputPath,
    [int]$Width = 1366,
    [int]$Height = 768,
    [int]$WaitSeconds = 4
)

$ErrorActionPreference = "Stop"

$chromePaths = @(
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    (Get-Command chrome.exe -ErrorAction SilentlyContinue).Source
) | Where-Object { $_ -and (Test-Path $_) }

if ($chromePaths.Count -eq 0) {
    throw "chrome.exe not found. Please install Google Chrome."
}
$chrome = $chromePaths | Select-Object -First 1
Write-Host "Using Chrome: $chrome"

$absOut = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
$dir = Split-Path -Parent $absOut
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }

$waitMs = $WaitSeconds * 1000

$argList = @(
    "--headless",
    "--disable-gpu",
    "--run-all-compositor-stages-before-draw",
    "--virtual-time-budget=$waitMs",
    "--screenshot=$absOut",
    "--window-size=${Width},${Height}",
    "--hide-scrollbars",
    "--no-sandbox",
    $Url
)

Start-Process -FilePath $chrome -ArgumentList $argList -Wait -NoNewWindow | Out-Null

if (-not (Test-Path $absOut)) {
    throw "Screenshot failed: $absOut was not created"
}

Write-Host "Saved $absOut"
