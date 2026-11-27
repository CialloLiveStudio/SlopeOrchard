param([Parameter(Mandatory = $true)][string]$Command)
$ErrorActionPreference = "Stop"
$available = Get-Command $Command -CommandType Application -ErrorAction SilentlyContinue
if ($available) { return $available[0].Source }
$wingetLink = Join-Path $env:LOCALAPPDATA "Microsoft/WinGet/Links/$Command.exe"
if (Test-Path -LiteralPath $wingetLink -PathType Leaf) { return $wingetLink }
throw "Could not locate $Command on PATH or in the WinGet links directory."
