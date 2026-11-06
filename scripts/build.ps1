# Build LingYan release binary.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $root '..\.cargo'
$env:RUSTUP_HOME = Join-Path $root '..\.rust'
$env:TMP = Join-Path $root '..\.tmp'
$env:TEMP = $env:TMP
Push-Location $root
try {
    & (Join-Path $env:CARGO_HOME 'bin\cargo.exe') build --release
} finally {
    Pop-Location
}
