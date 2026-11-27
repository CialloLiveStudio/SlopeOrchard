param(
    [string]$GodotConsole = "godot_console",
    [string]$Toolchain = "1.100.0-x86_64-pc-windows-msvc"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$cargo = Join-Path $env:USERPROFILE ".cargo/bin/cargo.exe"
$project = Join-Path $root "godot"
$console = & (Join-Path $PSScriptRoot "resolve-godot.ps1") $GodotConsole

function Invoke-Checked {
    param([string]$File, [string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File failed with exit code $LASTEXITCODE" }
}

Push-Location $root
try {
    $version = Invoke-Checked $console @("--version")
    if ($version -notmatch '^4\.7\.2\.stable') { throw "Godot 4.7.2 is required. Found $version." }
    Write-Output $version
    Invoke-Checked $cargo @("+$Toolchain", "build", "--release", "--locked", "-p", "slope-orchard-extension")
    Copy-Item "target/release/slope_orchard_extension.dll" "godot/native/slope_orchard_extension.dll" -Force
    Invoke-Checked $console @("--headless", "--path", $project, "--import", "--quit")
    Invoke-Checked $console @("--headless", "--path", $project, "--script", "res://scripts/probe.gd")
} finally {
    Pop-Location
}
