# OpDev verification extension for cargo-dist 0.32.0 (Apache-2.0).
function Test-OpdevSignedArchive([string]$Archive, [string]$ArchiveUrl) {
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Stop'
    $verificationDir = Join-Path ([IO.Path]::GetTempPath()) ('opdev-verification-' + [Guid]::NewGuid().ToString('N'))
    try {
        $null = New-Item -ItemType Directory -Path $verificationDir
        $verifier = Join-Path $verificationDir 'cosign.exe'
        Get-OpdevHttps "https://github.com/sigstore/cosign/releases/download/@COSIGN_VERSION@/cosign-windows-amd64.exe" $verifier
        if ((Get-FileHash -LiteralPath $verifier -Algorithm SHA256).Hash.ToLowerInvariant() -ne '@WINDOWS_DIGEST@') {
            throw 'Signature verifier checksum mismatch.'
        }
        $bundle = Join-Path $verificationDir 'bundle.json'
        Get-OpdevHttps "$ArchiveUrl.sigstore.json" $bundle
        Invoke-OpdevSignatureVerifier $verifier $Archive $bundle '@IDENTITY@'
    } finally {
        $ErrorActionPreference = $previousPreference
        Remove-Item -LiteralPath $verificationDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-OpdevSignatureVerifier([string]$Verifier, [string]$Archive, [string]$Bundle, [string]$Identity) {
    $previousPreference = $ErrorActionPreference
    try {
        # PowerShell 5.1 reports successful native stderr output as an error record.
        $ErrorActionPreference = 'Continue'
        & $Verifier verify-blob $Archive --bundle $Bundle --certificate-identity $Identity --certificate-oidc-issuer https://gitlab.com | ForEach-Object { [Console]::Error.WriteLine($_) }
        if ($LASTEXITCODE -ne 0) { throw 'Archive signature verification failed.' }
    } finally { $ErrorActionPreference = $previousPreference }
}

function Get-OpdevHttps([string]$Url, [string]$Path) {
    Add-Type -AssemblyName System.Net.Http
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [TimeSpan]::FromSeconds(180)
    try {
        $uri = [Uri]$Url
        for ($redirect = 0; $redirect -lt 10; $redirect++) {
            if ($uri.Scheme -ne 'https') { throw 'HTTPS is required, including redirects.' }
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
        throw 'Too many redirects.'
    } finally { $client.Dispose(); $handler.Dispose() }
}
