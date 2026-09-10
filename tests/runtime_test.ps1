# Offline PowerShell bootstrap tests. No Pester, network, or native CLI required.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../plugins/opdev/scripts/runtime.ps1')
$sourcePlugin = $OpdevPluginRoot
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('opdev runtime tests ' + [Guid]::NewGuid().ToString('N'))
$previousData = $env:OPDEV_DATA_DIR
$script:Cases = 0
function Assert-True($Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Get-OpdevPlatform { return @('Windows', 'X64') }
function New-Case {
    $script:Cases++
    $case = Join-Path $testRoot $script:Cases
    $script:OpdevPluginRoot = Join-Path $case 'plugin'
    $null = New-Item -ItemType Directory -Path $case -Force
    Copy-Item -LiteralPath $sourcePlugin -Destination $script:OpdevPluginRoot -Recurse
    $env:OPDEV_DATA_DIR = Join-Path $case 'data with spaces'
    $script:Fixture = Join-Path $case 'fixture'
    $null = New-Item -ItemType Directory -Path $script:Fixture
    [IO.File]::WriteAllText((Join-Path $script:Fixture 'verifier'), 'trusted fixture verifier')
    $digest = Get-OpdevHash (Join-Path $script:Fixture 'verifier')
    $path = Join-Path $script:OpdevPluginRoot 'runtime.lock'
    $lock = Get-Content -LiteralPath $path
    $lock = $lock | ForEach-Object { if ($_.StartsWith('target ')) { $_ -replace '[0-9a-f]{64}$', $digest } else { $_ } }
    [IO.File]::WriteAllLines($path, [string[]]$lock)
    [IO.File]::WriteAllText((Join-Path $script:Fixture 'bundle'), 'valid')
    $zip = [IO.Compression.ZipFile]::Open((Join-Path $script:Fixture 'archive'), [IO.Compression.ZipArchiveMode]::Create)
    try {
        $entry = $zip.CreateEntry('opdev.exe')
        $writer = [IO.StreamWriter]::new($entry.Open())
        try { $writer.Write('fixture CLI') } finally { $writer.Dispose() }
    } finally { $zip.Dispose() }
    $script:Downloads = 0
    $script:Verifications = 0
    $script:Executions = 0
    $script:FixtureVersion = '0.1.1'
    $script:Compatible = $true
}
function Invoke-OpdevDownload([string]$Url, [string]$Path) {
    $script:Downloads++
    if ($Url -like 'https://github.com/sigstore/cosign/releases/download/v3.1.3/cosign-*') { $file = 'verifier' }
    elseif ($Url -like 'https://gitlab.com/stolenfootball-tools/opdev/-/releases/v0.1.1/downloads/*.sigstore.json') { $file = 'bundle' }
    elseif ($Url -like 'https://gitlab.com/stolenfootball-tools/opdev/-/releases/v0.1.1/downloads/*.zip') { $file = 'archive' }
    else { throw "Unexpected download URL: $Url" }
    Copy-Item -LiteralPath (Join-Path $script:Fixture $file) -Destination $Path
}
function Invoke-OpdevNative([string]$Program, [string[]]$Arguments) {
    if ((Split-Path $Program -Leaf) -eq 'cosign.exe') {
        $script:Verifications++
        Assert-True ($Arguments[0] -eq 'verify-blob') 'Wrong verifier command'
        Assert-True ($Arguments[3] -and (Get-Content -LiteralPath $Arguments[3] -Raw) -eq 'valid') 'Signature failure'
        Assert-True ($Arguments[4] -eq '--certificate-identity') 'Missing identity constraint'
        Assert-True ($Arguments[5] -eq 'https://gitlab.com/stolenfootball-tools/opinionateddevelopment//.gitlab-ci.yml@refs/tags/v0.1.1') 'Wrong signature identity'
        Assert-True ($Arguments[6] -eq '--certificate-oidc-issuer' -and $Arguments[7] -eq 'https://gitlab.com') 'Wrong issuer'
        return
    }
    Assert-True ($script:Verifications -gt 0) 'Executed CLI before verification'
    $script:Executions++
    if ($Arguments[0] -eq 'version') { return @("opdev $script:FixtureVersion", 'project schema 1', 'rule catalog 1') }
    if ($Arguments[0] -eq 'plugin') { Assert-True $script:Compatible 'Compatibility failure'; return }
    return $Arguments
}
function Assert-Failure([scriptblock]$Action) {
    $failed = $false
    try { & $Action } catch { $failed = $true }
    Assert-True $failed 'Expected failure'
}
function Assert-CleanFailure {
    $files = @(Get-ChildItem -LiteralPath $env:OPDEV_DATA_DIR -Recurse -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -eq 'opdev.exe' -or $_.Name -like '.install.*' -or $_.Name -like '*.lock' })
    Assert-True ($files.Count -eq 0) 'Failed install left runtime, staging, or lock files'
}
try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-Case
    $binary = Invoke-OpdevRuntime 'Install' @()
    Assert-True (Test-Path -LiteralPath $binary) 'Runtime missing'
    $downloads = $script:Downloads
    Assert-True ((Invoke-OpdevRuntime 'Install' @()) -eq $binary) 'Repeat install changed path'
    Assert-True ((Invoke-OpdevRuntime 'Path' @()) -eq $binary) 'Path lookup differs'
    Assert-True ($script:Downloads -eq $downloads) 'Repeat install accessed network'
    $result = @(Invoke-OpdevRuntime 'Run' @('check', 'argument with spaces'))
    Assert-True ($result.Count -eq 2 -and $result[1] -eq 'argument with spaces') 'Arguments not preserved'

    New-Case
    Assert-Failure { Invoke-OpdevRuntime 'Path' @() }
    Assert-True (-not (Test-Path -LiteralPath $env:OPDEV_DATA_DIR)) 'Path lookup created storage'
    Assert-True ($script:Downloads -eq 0) 'Path lookup accessed network'

    New-Case
    [IO.File]::WriteAllText((Join-Path $script:Fixture 'verifier'), 'corrupt')
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-True ($script:Verifications -eq 0 -and $script:Executions -eq 0) 'Executed unverified download'
    Assert-CleanFailure

    New-Case
    [IO.File]::WriteAllText((Join-Path $script:Fixture 'bundle'), 'invalid')
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-True ($script:Executions -eq 0) 'Executed CLI after invalid signature'
    Assert-CleanFailure
    [IO.File]::WriteAllText((Join-Path $script:Fixture 'bundle'), 'valid')
    $null = Invoke-OpdevRuntime 'Install' @()

    New-Case
    $script:FixtureVersion = '9.9.9'
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-CleanFailure

    New-Case
    $script:Compatible = $false
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-CleanFailure

    New-Case
    $binary = Invoke-OpdevRuntime 'Install' @()
    [IO.File]::WriteAllText($binary, 'corrupted')
    $executions = $script:Executions
    $downloads = $script:Downloads
    Assert-Failure { Invoke-OpdevRuntime 'Run' @('version') }
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-True ($script:Executions -eq $executions -and $script:Downloads -eq $downloads) 'Damaged cache was executed or silently replaced'

    New-Case
    $binary = Invoke-OpdevRuntime 'Install' @()
    $destination = Split-Path $binary -Parent
    Remove-Item -LiteralPath $destination -Recurse -Force
    [IO.File]::WriteAllText("$destination.lock", 'in progress')
    Assert-Failure { Invoke-OpdevRuntime 'Install' @() }
    Assert-True (Test-Path -LiteralPath "$destination.lock") 'Removed another installer lock'
    Write-Output "$script:Cases PowerShell installer cases passed."
} finally {
    $env:OPDEV_DATA_DIR = $previousData
    if (Test-Path -LiteralPath $testRoot) { Remove-Item -LiteralPath $testRoot -Recurse -Force }
}
