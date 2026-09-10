param([Parameter(Mandatory = $true)][string]$Installer)
$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('opdev dist tests ' + [Guid]::NewGuid().ToString('N'))
$savedUnmanaged = $env:OPDEV_UNMANAGED_INSTALL
try {
    $null = New-Item -ItemType Directory -Path $root
    $assets = Split-Path (Resolve-Path $Installer) -Parent
    $fixtureVerifier = Join-Path $root 'verifier'
    [IO.File]::WriteAllText($fixtureVerifier, 'fixture verifier')
    $digest = (Get-FileHash -LiteralPath $fixtureVerifier -Algorithm SHA256).Hash.ToLowerInvariant()
    $body = Get-Content -LiteralPath $Installer -Raw
    $tokens = $null; $parseErrors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseInput($body, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count) { throw 'Generated PowerShell installer has syntax errors' }
    $body = $body.Substring(0, $body.IndexOf('# The default interactive handler'))
    $body = $body.Replace('9fe59be0eca1271873ce019061335eb1ac419b7059202e797828467ddabe33be', $digest)
    $bootstrap = Join-Path $root 'fixture.ps1'
    [IO.File]::WriteAllText($bootstrap, $body)
    $env:OPDEV_UNMANAGED_INSTALL = Join-Path $root 'installed with spaces'
    . $bootstrap
    function Get-TargetTriple($platforms) { return 'x86_64-pc-windows-msvc' }
    function Invoke-DownloadFile($client, $url, $path) {
        if (-not $url.StartsWith('https://github.com/stolenfootball/opdev/releases/download/v0.1.2/')) { throw 'Wrong archive host' }
        Copy-Item -LiteralPath (Join-Path $assets ($url.Split('/')[-1])) -Destination $path
    }
    function Get-OpdevHttps([string]$Url, [string]$Path) {
        if ($Url -eq 'https://github.com/sigstore/cosign/releases/download/v3.1.3/cosign-windows-amd64.exe') {
            Copy-Item -LiteralPath $fixtureVerifier -Destination $Path
        } elseif ($Url -eq 'https://github.com/stolenfootball/opdev/releases/download/v0.1.2/opdev-x86_64-pc-windows-msvc.zip.sigstore.json') {
            [IO.File]::WriteAllText($Path, $script:BundleState)
        } else { throw 'Unexpected verification download' }
    }
    function Invoke-OpdevSignatureVerifier([string]$Verifier, [string]$Archive, [string]$Bundle, [string]$Identity) {
        $script:Verified++
        if ($Identity -ne 'https://gitlab.com/stolenfootball-tools/opdev//.gitlab-ci.yml@refs/tags/v0.1.2') { throw 'Wrong signing identity' }
        if ((Get-Content -LiteralPath $Bundle -Raw) -ne 'valid') { throw 'Invalid fixture signature' }
    }
    $script:Verified = 0
    $script:BundleState = 'valid'
    Install-Binary @()
    $installed = Join-Path $env:OPDEV_UNMANAGED_INSTALL 'opdev.exe'
    if (-not (Test-Path -LiteralPath $installed) -or $script:Verified -ne 1) { throw 'Install did not verify first' }
    Install-Binary @()
    if ($script:Verified -ne 2) { throw 'Repeat install skipped verification' }
    Remove-Item -LiteralPath $env:OPDEV_UNMANAGED_INSTALL -Recurse -Force
    foreach ($failure in @('signature', 'verifier')) {
        $script:BundleState = 'invalid'
        if ($failure -eq 'verifier') { [IO.File]::WriteAllText($fixtureVerifier, 'corrupt') }
        $failed = $false
        try { Install-Binary @() } catch { $failed = $true }
        if (-not $failed -or (Test-Path -LiteralPath $installed)) { throw 'Installed without valid verification' }
    }
    if ($script:Verified -ne 3) { throw 'Executed a corrupt verifier' }
    Write-Output 'Generated PowerShell installation, repeat use, invalid signature, and corrupt verifier checks passed.'
} finally {
    $env:OPDEV_UNMANAGED_INSTALL = $savedUnmanaged
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
