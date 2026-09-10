# Live Windows consumer check against the lock's existing signed release.
$ErrorActionPreference = 'Stop'
$previous = $env:OPDEV_DATA_DIR
$root = Join-Path ([IO.Path]::GetTempPath()) ('opdev-live-' + [Guid]::NewGuid().ToString('N'))
try {
    $env:OPDEV_DATA_DIR = $root
    $bootstrap = Join-Path $PSScriptRoot '../plugins/opdev/scripts/runtime.ps1'
    $binary = & $bootstrap -Mode Install
    if ($LASTEXITCODE -ne 0) { throw 'Live runtime install failed' }
    & $bootstrap -Mode Run version
    if ($LASTEXITCODE -ne 0) { throw 'Installed CLI failed' }
    $again = & $bootstrap -Mode Install
    if ($LASTEXITCODE -ne 0 -or $again -cne $binary) { throw 'Repeated install changed runtime' }
} finally {
    $env:OPDEV_DATA_DIR = $previous
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
