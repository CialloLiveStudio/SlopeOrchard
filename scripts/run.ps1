param([string]$Godot = "godot", [string]$GodotConsole = "godot_console")
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$project = Join-Path $root "godot"
$dll = Get-Item (Join-Path $project "native/slope_orchard_extension.dll") -ErrorAction SilentlyContinue
$rustInputs = Get-ChildItem (Join-Path $root "rust") -Recurse -File
$buildInputs = Get-Item (Join-Path $root "Cargo.toml"), (Join-Path $root "Cargo.lock"), (Join-Path $root "rust-toolchain.toml")
$stale = $dll -and (($rustInputs + $buildInputs) | Where-Object LastWriteTimeUtc -gt $dll.LastWriteTimeUtc)
if (!$dll -or $stale -or !(Test-Path (Join-Path $project ".godot/extension_list.cfg"))) {
    & (Join-Path $PSScriptRoot "build.ps1") -GodotConsole $GodotConsole
}
$engine = & (Join-Path $PSScriptRoot "resolve-godot.ps1") $Godot
Start-Process -FilePath $engine -WorkingDirectory $project -ArgumentList @("--path", ('"{0}"' -f $project))
