param([ValidateSet('format', 'lint')][string]$Mode)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $projectRoot 'target/style-tools/PSScriptAnalyzer/1.25.0/PSScriptAnalyzer.psd1') -Force
$settings = Join-Path $PSScriptRoot 'PSScriptAnalyzerSettings.psd1'
$paths = @('scripts', 'tests', 'plugins', 'release') | ForEach-Object {
    Get-ChildItem -LiteralPath (Join-Path $projectRoot $_) -Recurse -File | Where-Object { $_.Extension -in @('.ps1', '.psd1') }
}
$failed = $false
foreach ($path in $paths) {
    if ($Mode -eq 'format') {
        $original = [IO.File]::ReadAllText($path.FullName)
        $formatted = Invoke-Formatter -ScriptDefinition $original -Settings $settings
        if ($original.Replace("`r`n", "`n") -cne $formatted.Replace("`r`n", "`n")) {
            Write-Output "PowerShell formatting differs: $($path.FullName)"
            $failed = $true
        }
    } else {
        $findings = @(Invoke-ScriptAnalyzer -Path $path.FullName -Settings $settings)
        if ($findings.Count -gt 0) {
            $findings | Format-Table -AutoSize
            $failed = $true
        }
    }
}
if ($failed) { exit 1 }
