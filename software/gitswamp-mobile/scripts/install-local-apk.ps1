[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$apk = Join-Path $projectRoot "dist\GitSwamp-Mobile-local.apk"

if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot "build-local-apk.ps1")
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$localAdb = Join-Path $projectRoot ".android-sdk\platform-tools\adb.exe"
$adbCommand = if (Test-Path -LiteralPath $localAdb) {
    $localAdb
} else {
    (Get-Command adb -ErrorAction Stop).Source
}

$devices = & $adbCommand devices
$connected = @($devices | Select-Object -Skip 1 | Where-Object { $_ -match "\sdevice$" })
if ($connected.Count -eq 0) {
    throw "No authorized Android device found. Enable USB debugging, connect the phone, and accept its authorization prompt."
}

if (-not (Test-Path -LiteralPath $apk)) {
    throw "APK not found at $apk. Run build-local-apk.ps1 first."
}

& $adbCommand install -r $apk
if ($LASTEXITCODE -ne 0) {
    throw "ADB installation failed with exit code $LASTEXITCODE."
}

Write-Host "GitSwamp Mobile is installed."
