[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$sourceApk = Join-Path $projectRoot "app\build\outputs\apk\local\app-local.apk"
$debugSourceApk = Join-Path $projectRoot "app\build\outputs\apk\debug\app-debug.apk"
$distDirectory = Join-Path $projectRoot "dist"
$targetApk = Join-Path $distDirectory "GitSwamp-Mobile-local.apk"
$debugTargetApk = Join-Path $distDirectory "GitSwamp-Mobile-debug.apk"
$signingDirectory = Join-Path $projectRoot "signing"
$keystore = Join-Path $signingDirectory "gitswamp-mobile-local.keystore"
$passwordFile = Join-Path $signingDirectory "keystore-password.txt"
$alignedApk = Join-Path $distDirectory "aligned-local.apk"
$sdkRoot = Join-Path $projectRoot ".android-sdk"
$buildTools = Get-ChildItem (Join-Path $sdkRoot "build-tools") -Directory | Sort-Object Name -Descending | Select-Object -First 1
$keytool = Join-Path $env:JAVA_HOME "bin\keytool.exe"
$zipalign = Join-Path $buildTools.FullName "zipalign.exe"
$apksigner = Join-Path $buildTools.FullName "apksigner.bat"

Push-Location $projectRoot
try {
    & ".\gradlew.bat" testDebugUnitTest lintDebug assembleDebug assembleLocal
    if ($LASTEXITCODE -ne 0) {
        throw "Gradle verification failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $sourceApk)) {
        throw "Expected APK was not created at $sourceApk."
    }
    if (-not (Test-Path -LiteralPath $debugSourceApk)) {
        throw "Expected debug APK was not created at $debugSourceApk."
    }

    New-Item -ItemType Directory -Path $distDirectory -Force | Out-Null
    New-Item -ItemType Directory -Path $signingDirectory -Force | Out-Null
    if (-not (Test-Path -LiteralPath $keystore)) {
        $password = [guid]::NewGuid().ToString("N")
        Set-Content -LiteralPath $passwordFile -Value $password
        & $keytool -genkeypair -keystore $keystore -storepass $password -keypass $password -alias gitswamp-mobile-local -keyalg RSA -keysize 3072 -validity 3650 -dname "CN=GitSwamp Mobile Local, OU=GitSwamp, O=GitSwamp, C=HR"
        if ($LASTEXITCODE -ne 0) { throw "Could not create the local signing key." }
    }
    $password = (Get-Content -LiteralPath $passwordFile -Raw).Trim()
    & $zipalign -f -p 4 $sourceApk $alignedApk
    if ($LASTEXITCODE -ne 0) { throw "APK alignment failed." }
    & $apksigner sign --ks $keystore --ks-key-alias gitswamp-mobile-local --ks-pass "file:$passwordFile" --key-pass "pass:$password" --out $targetApk $alignedApk
    if ($LASTEXITCODE -ne 0) { throw "APK signing failed." }
    Remove-Item -LiteralPath $alignedApk -Force
    Copy-Item -LiteralPath $debugSourceApk -Destination $debugTargetApk -Force

    $apk = Get-Item -LiteralPath $targetApk
    $hash = Get-FileHash -LiteralPath $targetApk -Algorithm SHA256
    Write-Host "APK: $($apk.FullName)"
    Write-Host ("Size: {0:N1} MB" -f ($apk.Length / 1MB))
    Write-Host "SHA-256: $($hash.Hash)"
    Write-Host "ADB debug APK: $debugTargetApk"
}
finally {
    Pop-Location
}
