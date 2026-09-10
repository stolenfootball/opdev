# Dot-source for tests; invoking the script runs the requested operation.
param(
    [ValidateSet('Path', 'Install', 'Run')][string]$Mode = 'Path',
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$CliArgs
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:OpdevExitCode = 1
$OpdevPluginRoot = Split-Path $PSScriptRoot -Parent

function Get-OpdevPlatform {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        throw 'On macOS and Linux use runtime.sh.'
    }
    return @('Windows', [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString())
}
function Get-OpdevHash([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Invoke-OpdevDownload([string]$Url, [string]$Path) {
    if (-not $Url.StartsWith('https://')) { throw 'Downloads require HTTPS.' }
    Add-Type -AssemblyName System.Net.Http
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [TimeSpan]::FromSeconds(180)
    try {
        $uri = [Uri]$Url
        for ($redirect = 0; $redirect -lt 10; $redirect++) {
            if ($uri.Scheme -ne 'https') { throw 'HTTPS downgrade redirect rejected.' }
            $response = $client.GetAsync($uri).GetAwaiter().GetResult()
            try {
                if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
                    if (-not $response.Headers.Location) { throw 'Missing redirect location.' }
                    $uri = [Uri]::new($uri, $response.Headers.Location)
                    continue
                }
                $null = $response.EnsureSuccessStatusCode()
                [IO.File]::WriteAllBytes($Path, $response.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult())
                return
            } finally { $response.Dispose() }
        }
        throw 'Too many download redirects.'
    } finally { $client.Dispose(); $handler.Dispose() }
}
function Invoke-OpdevNative([string]$Program, [string[]]$Arguments) {
    # Windows PowerShell 5.1 can treat a successful native stderr write as an
    # ErrorAction exception. Judge the native process by its exit code.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $result = & $Program @Arguments
        $nativeExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($nativeExit -ne 0) { throw "Native command failed (exit $nativeExit): $Program" }
    return $result
}
function Test-OpdevRuntime([string]$Destination) {
    $binary = Join-Path $Destination 'opdev.exe'
    $receipt = Join-Path $Destination 'opdev.sha256'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf) -or -not (Test-Path -LiteralPath $receipt -PathType Leaf)) { return $false }
    foreach ($path in @($Destination, $binary, $receipt)) {
        if ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { return $false }
    }
    return (Get-OpdevHash $binary) -eq (Get-Content -LiteralPath $receipt -Raw).Trim()
}
function Invoke-OpdevRuntime([string]$Operation, [string[]]$Arguments) {
    $rows = @(Get-Content -LiteralPath (Join-Path $OpdevPluginRoot 'runtime.lock') | Where-Object { $_ -and -not $_.StartsWith('#') })
    $values = @{}
    foreach ($line in $rows) {
        $parts = $line -split '\s+'
        if ($parts[0] -ne 'target') { $values[$parts[0]] = $parts[1] }
    }
    $version = $values['version']; $tag = $values['tag']; $identity = $values['identity']
    if ($tag -cne "v$version" -or $tag -notmatch '^v[0-9A-Za-z.-]+$') { throw 'Invalid runtime version lock.' }
    $platform = Get-OpdevPlatform
    $matches = @($rows | Where-Object { $p = $_ -split '\s+'; $p[0] -eq 'target' -and $p[1] -eq $platform[0] -and $p[2] -eq $platform[1] })
    if ($matches.Count -ne 1) { throw "Unsupported platform: $platform" }
    $target = $matches[0] -split '\s+'
    $triple = $target[3]; $verifierAsset = $target[4]; $verifierDigest = $target[5]
    if ($triple -notmatch '^[A-Za-z0-9_-]+$') { throw 'Invalid target lock.' }
    $data = $env:OPDEV_DATA_DIR
    if (-not $data) { $data = Join-Path $env:LOCALAPPDATA 'opdev' }
    if (-not [IO.Path]::IsPathRooted($data)) { throw 'Runtime data directory must be absolute.' }
    $parent = Join-Path (Join-Path $data 'runtimes') $tag
    $destination = Join-Path $parent $triple
    $binary = Join-Path $destination 'opdev.exe'
    $contract = Join-Path $OpdevPluginRoot 'opdev-compatibility.json'
    if ($Operation -ne 'Install') {
        if (-not (Test-Path -LiteralPath $destination)) {
            $script:OpdevExitCode = 3
            throw 'No managed runtime installed. Run runtime.ps1 -Mode Install.'
        }
        if (-not (Test-OpdevRuntime $destination)) { throw 'Managed CLI is missing or damaged. Run runtime.ps1 -Mode Install.' }
        if ($Operation -eq 'Path') { return $binary }
        $null = Invoke-OpdevNative $binary @('plugin', 'verify', '--contract', $contract)
        return Invoke-OpdevNative $binary $Arguments
    }
    if (Test-Path -LiteralPath $destination) {
        if (-not (Test-OpdevRuntime $destination)) { throw "Existing runtime is damaged; move aside this directory before retrying: $destination" }
        $null = Invoke-OpdevNative $binary @('plugin', 'verify', '--contract', $contract)
        return $binary
    }
    $null = [IO.Directory]::CreateDirectory($parent)
    $lockPath = "$destination.lock"
    try { $installLock = [IO.File]::Open($lockPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None) }
    catch { throw "Another setup may be running. If interrupted, remove the stale lock: $lockPath" }
    $staging = Join-Path $parent ('.install.' + [Guid]::NewGuid().ToString('N'))
    try {
        if (Test-Path -LiteralPath $destination) { throw 'Runtime appeared during setup; retry.' }
        $null = [IO.Directory]::CreateDirectory($staging)
        $verifier = Join-Path $staging 'cosign.exe'
        [Console]::Error.WriteLine("Downloading pinned signature verifier $($values['cosign'])...")
        Invoke-OpdevDownload "https://github.com/sigstore/cosign/releases/download/$($values['cosign'])/$verifierAsset" $verifier
        if ((Get-OpdevHash $verifier) -cne $verifierDigest) { throw 'Signature verifier checksum mismatch.' }
        $archive = "opdev-$version-$triple.zip"
        $base = "https://gitlab.com/stolenfootball-tools/opdev/-/releases/$tag/downloads"
        $archivePath = Join-Path $staging $archive
        $bundlePath = "$archivePath.sigstore.json"
        [Console]::Error.WriteLine("Downloading and verifying OpDev $version for $triple...")
        Invoke-OpdevDownload "$base/$archive" $archivePath
        Invoke-OpdevDownload "$base/$archive.sigstore.json" $bundlePath
        $null = Invoke-OpdevNative $verifier @('verify-blob', $archivePath, '--bundle', $bundlePath, '--certificate-identity', $identity, '--certificate-oidc-issuer', 'https://gitlab.com')
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($archivePath)
        $runtime = Join-Path $staging 'runtime'
        $null = [IO.Directory]::CreateDirectory($runtime)
        try {
            $entries = @($zip.Entries | Where-Object { $_.FullName -ceq 'opdev.exe' })
            if ($entries.Count -ne 1) { throw 'Archive must contain exactly one opdev.exe entry.' }
            $entryType = ($entries[0].ExternalAttributes -shr 16) -band 0xF000
            if ($entryType -ne 0 -and $entryType -ne 0x8000) { throw 'CLI archive entry is not a regular file.' }
            # Never expand archive-controlled paths. Copy only the exact executable.
            $inputStream = $entries[0].Open()
            try {
                $outputStream = [IO.File]::Create((Join-Path $runtime 'opdev.exe'))
                try { $inputStream.CopyTo($outputStream) } finally { $outputStream.Dispose() }
            } finally { $inputStream.Dispose() }
        } finally { $zip.Dispose() }
        $stagedBinary = Join-Path $runtime 'opdev.exe'
        $versionOutput = @(Invoke-OpdevNative $stagedBinary @('version'))
        if ($versionOutput.Count -eq 0 -or $versionOutput[0] -cne "opdev $version") { throw 'Downloaded CLI version does not match the pin.' }
        $null = Invoke-OpdevNative $stagedBinary @('plugin', 'verify', '--contract', $contract)
        [IO.File]::WriteAllText((Join-Path $runtime 'opdev.sha256'), (Get-OpdevHash $stagedBinary))
        [IO.Directory]::Move($runtime, $destination)
        return $binary
    } finally {
        if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
        $installLock.Dispose()
        Remove-Item -LiteralPath $lockPath -Force
    }
}
if ($MyInvocation.InvocationName -ne '.') {
    try { Invoke-OpdevRuntime $Mode $CliArgs }
    catch { [Console]::Error.WriteLine("OpDev setup: $($_.Exception.Message)"); exit $script:OpdevExitCode }
}
