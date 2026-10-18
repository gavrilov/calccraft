<#
.SYNOPSIS
Builds the Android app as an APK for 64-bit ARM phones and tablets.

.DESCRIPTION
Builds the Rust library for Android with cargo-ndk, then packages it with the Android SDK's
own tools, without Gradle: aapt2 (manifest and resources), zipalign (16 KB pages for the
uncompressed library, as Google Play requires) and apksigner. The APK is signed with the
debug key (~/.android/debug.keystore, created when missing), which is fine for `adb install`
but not for Google Play. See android/README.md.

.EXAMPLE
pwsh android/build-apk.ps1
adb install -r target/android/CalcCraft-1.2.0-arm64.apk
#>
param(
    [string]$Sdk = $(if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }),
    [string]$Ndk = '29.0.14206865',
    [string]$BuildTools = '36.0.0',
    [string]$Platform = 'android-36',
    [string]$Jdk = $(if ($env:JAVA_HOME) { $env:JAVA_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\jdk-21' }),
    [string]$Keystore = (Join-Path $HOME '.android\debug.keystore')
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repo = Split-Path $PSScriptRoot
$out = Join-Path $repo 'target\android'
$tools = Join-Path $Sdk "build-tools\$BuildTools"
$env:ANDROID_HOME = $Sdk
$env:ANDROID_NDK_HOME = Join-Path $Sdk "ndk\$Ndk"
$env:JAVA_HOME = $Jdk
# Android 7.0, the oldest version Android's developer verification covers.
$minSdk = 24
$targetSdk = 36

# The version from Cargo.toml: 1.2.0 is version code 10200.
$version = (Select-String -Path (Join-Path $repo 'Cargo.toml') -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$parts = $version.Split('.') | ForEach-Object { [int]$_ }
$versionCode = $parts[0] * 10000 + $parts[1] * 100 + $parts[2]

Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
$staging = Join-Path $out 'staging'
$libDir = Join-Path $staging 'lib\arm64-v8a'
New-Item -ItemType Directory -Force $libDir | Out-Null

# The Rust library: a cdylib only for Android (desktops and the web build the binary).
Push-Location $repo
try {
    cargo ndk -t arm64-v8a -P $minSdk rustc --lib --release --crate-type cdylib
} finally {
    Pop-Location
}
$strip = Join-Path $env:ANDROID_NDK_HOME 'toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-strip.exe'
& $strip --strip-unneeded -o (Join-Path $libDir 'libcalculator.so') (Join-Path $repo 'target\aarch64-linux-android\release\libcalculator.so')

# Manifest and resources (the launcher icon and the themes).
$resources = Join-Path $out 'resources.zip'
& (Join-Path $tools 'aapt2.exe') compile --dir (Join-Path $PSScriptRoot 'res') -o $resources
$unaligned = Join-Path $out 'unaligned.apk'
& (Join-Path $tools 'aapt2.exe') link -o $unaligned `
    -I (Join-Path $Sdk "platforms\$Platform\android.jar") `
    --manifest (Join-Path $PSScriptRoot 'AndroidManifest.xml') `
    --min-sdk-version $minSdk --target-sdk-version $targetSdk `
    --version-code $versionCode --version-name $version `
    $resources

# The library goes in uncompressed (stored), so Android maps it straight from the APK
# (`extractNativeLibs="false"`): .NET stores an entry added without compression, while
# `aapt add` compresses it whatever `-0` says.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open($unaligned, 'Update')
try {
    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip,
        (Join-Path $libDir 'libcalculator.so'), 'lib/arm64-v8a/libcalculator.so',
        [System.IO.Compression.CompressionLevel]::NoCompression) | Out-Null
} finally {
    $zip.Dispose()
}

$aligned = Join-Path $out 'aligned.apk'
& (Join-Path $tools 'zipalign.exe') -f -P 16 4 $unaligned $aligned

if (-not (Test-Path $Keystore)) {
    New-Item -ItemType Directory -Force (Split-Path $Keystore) | Out-Null
    & (Join-Path $Jdk 'bin\keytool.exe') -genkeypair -keystore $Keystore -storepass android `
        -keypass android -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 `
        -dname 'CN=Android Debug,O=Android,C=US'
}
$apk = Join-Path $out "CalcCraft-$version-arm64.apk"
& (Join-Path $tools 'apksigner.bat') sign --ks $Keystore --ks-pass pass:android `
    --key-pass pass:android --ks-key-alias androiddebugkey --out $apk $aligned
& (Join-Path $tools 'apksigner.bat') verify $apk
Remove-Item $unaligned, $aligned, $resources, $staging -Recurse -Force
"{0} ({1:N1} MB)" -f $apk, ((Get-Item $apk).Length / 1MB)
