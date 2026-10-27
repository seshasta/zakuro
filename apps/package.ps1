# Packages one of the folders here as a zip to hand to players: a folder
# made into a game of its own (see crates/zakuro/src/app.rs), with the program
# named after the game, the folder's own files, Zakuro's licence and the
# notices of what the program is built from. Nothing from a game or a
# console goes in; setup asks the player for those on the first run.
#
#   apps\package.ps1                         packages tomodachi-life
#   apps\package.ps1 -App tomodachi-life -Version 0.1
#
# The zip lands in target\dist.
param(
    [string] $App = 'tomodachi-life',
    [string] $Version = ''
)
$ErrorActionPreference = 'Stop'

# runs a program, which may write its progress to stderr as cargo does,
# and stops when it fails. Windows PowerShell takes stderr for an error
# while stopping on errors
function Run([scriptblock] $command) {
    $ErrorActionPreference = 'Continue'
    $output = & $command 2>&1 | ForEach-Object { if ($_ -is [System.Management.Automation.ErrorRecord]) { Write-Host $_.ToString(); $null } else { $_ } }
    if ($LASTEXITCODE) { throw "failed: $command" }
    $output
}

$repo = Split-Path $PSScriptRoot
$source = Join-Path $PSScriptRoot $App
if (-not (Test-Path (Join-Path $source 'app.toml'))) { throw "$source has no app.toml" }
$manifest = Get-Content (Join-Path $source 'app.toml') -Raw
$name = if ($manifest -match '(?m)^\s*name\s*=\s*"([^"]+)"') { $Matches[1] } else { $App }
if (-not $Version) { $Version = (Run { git -C $repo describe --always --dirty }).Trim() }
$out = Join-Path $repo 'target\dist'
$build = Join-Path $out 'build'

# the C runtime built into the program, so that it starts on a PC without
# the Visual C++ redistributable; built apart from the usual builds, which
# keep theirs
$env:RUSTFLAGS = '-C target-feature=+crt-static'
try {
    Run { cargo build --release -p zakuro --target-dir $build } | Out-Null
} finally {
    Remove-Item Env:RUSTFLAGS
}

$stage = Join-Path $out $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
Copy-Item -Recurse $source $stage
Copy-Item (Join-Path $build 'release\zakuro.exe') (Join-Path $stage "$name.exe")
Copy-Item (Join-Path $repo 'LICENSE') (Join-Path $stage 'LICENSE (Zakuro).txt')
# Windows' line endings in the text files, whichever a checkout gave them
foreach ($file in Get-ChildItem -Recurse $stage -File -Include *.txt, *.toml, *.bat, *.ps1) {
    $text = [IO.File]::ReadAllText($file.FullName).Replace("`r`n", "`n").Replace("`n", "`r`n")
    [IO.File]::WriteAllText($file.FullName, $text)
}

# the notices of every crate built into the program, for Windows, with the
# licence files each one carries
# read with .NET's reader, Windows PowerShell's turns down the keys
# cargo's metadata has that differ only in case
Add-Type -AssemblyName System.Web.Extensions
$json = New-Object System.Web.Script.Serialization.JavaScriptSerializer
$json.MaxJsonLength = [int]::MaxValue
$metadata = $json.DeserializeObject(((Run { cargo metadata --format-version 1 --manifest-path (Join-Path $repo 'Cargo.toml') }) -join "`n"))
$members = @($metadata['workspace_members'])
$packages = @{}
foreach ($package in $metadata['packages']) { $packages["$($package['name']) $($package['version'])"] = $package }
$tree = Run { cargo tree -p zakuro -e normal --target x86_64-pc-windows-msvc --prefix none --format '{p}' --manifest-path (Join-Path $repo 'Cargo.toml') }
$used = $tree | ForEach-Object { $words = $_ -split ' '; "$($words[0]) $($words[1].TrimStart('v'))" } | Sort-Object -Unique
$notices = New-Object System.Text.StringBuilder
[void] $notices.AppendLine("The program is built from these, besides Zakuro itself, under their own licences.")
[void] $notices.AppendLine('')
$mit = (Get-Content (Join-Path $repo 'LICENSE') -Raw) -replace '(?m)^Copyright.*$', ''
foreach ($key in $used) {
    $package = $packages[$key]
    if (-not $package -or $members -contains $package['id']) { continue }
    $dir = Split-Path $package['manifest_path']
    [void] $notices.AppendLine(('=' * 78))
    [void] $notices.AppendLine("$($package['name']) $($package['version']), $($package['license'])")
    if ($package['repository']) { [void] $notices.AppendLine($package['repository']) }
    $files = Get-ChildItem $dir -File | Where-Object { $_.Name -match '^(LICEN[CS]E|COPYING|NOTICE)' }
    if ($files) {
        foreach ($file in $files) {
            [void] $notices.AppendLine('')
            [void] $notices.AppendLine("--- $($file.Name)")
            [void] $notices.AppendLine((Get-Content $file.FullName -Raw).TrimEnd())
        }
    } elseif ($package['license'] -match 'MIT') {
        # a crate that names MIT without carrying its text
        $authors = if ($package['authors']) { $package['authors'] -join ', ' } else { "the $($package['name']) authors" }
        [void] $notices.AppendLine('')
        [void] $notices.AppendLine("Copyright (c) $authors")
        [void] $notices.AppendLine($mit.Trim())
    }
    [void] $notices.AppendLine('')
}
[IO.File]::WriteAllText((Join-Path $stage 'THIRD-PARTY-NOTICES.txt'), $notices.ToString().Replace("`r`n", "`n").Replace("`n", "`r`n"))

$zip = Join-Path $out "$name $Version.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Host "packaged $zip"
