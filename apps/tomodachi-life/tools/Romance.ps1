# Sets up a confession or a proposal between two islanders in Tomodachi Life's
# save (US), so it happens the next time the game is started.
#
#   Romance.ps1                              asks which islanders and what
#   Romance.ps1 -Action list
#   Romance.ps1 -Action confess -From 2 -To 0
#   Romance.ps1 -Action propose -From 2 -To 0
#
# It lives in the app folder's tools\ and edits ..\user unless -User says
# otherwise. "Set up a romance.bat" runs it.
#
# Where things are in savedataArc.txt (from the save's layout as Tomodachi
# Life Save Editor documents it, github.com/Brionjv/Tomodachi-Life-Save-Editor,
# and the problem names in the game's Trouble_Data message file):
#   0x10                 when the game last saved, seconds since 2000
#   0x1C70 + i*0x660     islander i's Mii data: +0x18 bit 0 is female,
#                        +0x1A the name, 10 UTF-16 characters
#   0x299F0 + i*0x100    islander i's relationships:
#     +j                 how i feels about j, 0 to 200, 100 to start with
#     +0x64 + j          what i and j are to each other (kept the same both
#                        ways): 0 strangers, 1 friends, 2 sweethearts, 3 exes,
#                        4 married, 6 divorced, 7 parent and child,
#                        8 siblings, 12 best friends
#     +0xC8              when i's current problem goes away, seconds since 2000
#     +0xD0              i's problem: 0x14C "I've got feelings for ...",
#                        0x154 "I want to propose to ...", with +0xD2 = 3
#     +0xD4, +0xD6       who the problem is about, 0xFFFF for no one
param(
    [ValidateSet('ask', 'list', 'confess', 'propose')] [string] $Action = 'ask',
    [int] $From = -1,
    [int] $To = -1,
    [string] $User = (Join-Path (Split-Path $PSScriptRoot) 'user')
)
$ErrorActionPreference = 'Stop'

function Fail([string] $message) {
    Write-Host ''
    Write-Host $message -ForegroundColor Red
    Write-Host 'Nothing was changed.'
    exit 1
}

$save = Join-Path $User 'savedata\000400000008C300\savedataArc.txt'
if (-not (Test-Path $save)) { Fail "No Tomodachi Life save at $save. Start the game once and save first." }
$data = [IO.File]::ReadAllBytes($save)
if ($data.Length -ne 1985688 -or $data[0] -ne 0x11) { Fail "$save doesn't look like a Tomodachi Life (US) save." }

$kinds = @{ 0 = 'strangers'; 1 = 'friends'; 2 = 'sweethearts'; 3 = 'exes'; 4 = 'married'; 5 = 'married';
            6 = 'divorced'; 7 = 'parent and child'; 8 = 'siblings'; 9 = 'friends (fighting)';
            10 = 'sweethearts (fighting)'; 11 = 'married (fighting)'; 12 = 'best friends' }

function Mii([int] $i) { 0x1C70 + $i * 0x660 }
function Rel([int] $i) { 0x299F0 + $i * 0x100 }
function Name([int] $i) {
    $name = [Text.Encoding]::Unicode.GetString($data, (Mii $i) + 0x1A, 20)
    $end = $name.IndexOf([char]0)
    if ($end -ge 0) { $name.Substring(0, $end) } else { $name }
}
function Female([int] $i) { ($data[(Mii $i) + 0x18] -band 1) -eq 1 }
function Kind([int] $i, [int] $j) { $data[(Rel $i) + 0x64 + $j] }
function KindName([int] $k) { if ($kinds.ContainsKey($k)) { $kinds[$k] } else { "unknown ($k)" } }
function SetKind([int] $i, [int] $j, [int] $k) { $data[(Rel $i) + 0x64 + $j] = $k; $data[(Rel $j) + 0x64 + $i] = $k }
function SetFeeling([int] $i, [int] $j, [int] $v) { $data[(Rel $i) + $j] = $v }
function U32([int] $at, [uint32] $v) { [BitConverter]::GetBytes($v).CopyTo($data, $at) }
function U16([int] $at, [uint16] $v) { [BitConverter]::GetBytes($v).CopyTo($data, $at) }

$islanders = @(0..99 | Where-Object { (Name $_) -ne '' })

function ShowIslanders {
    Write-Host ''
    Write-Host 'Islanders:'
    foreach ($i in $islanders) {
        $partner = ''
        foreach ($j in $islanders) {
            $k = Kind $i $j
            if ($j -ne $i -and @(2, 4, 5, 10, 11) -contains $k) { $partner = " - $(KindName $k) with $(Name $j)" }
        }
        $sex = if (Female $i) { 'F' } else { 'M' }
        Write-Host ('  {0,3}  {1,-10} {2}{3}' -f $i, (Name $i), $sex, $partner)
    }
    Write-Host ''
}

# someone i is sweethearts or married with, other than j, or -1
function OtherPartner([int] $i, [int] $j) {
    foreach ($o in $islanders) {
        if ($o -ne $i -and $o -ne $j -and @(2, 4, 5, 10, 11) -contains (Kind $i $o)) { return $o }
    }
    -1
}

function Ask([string] $question) {
    while ($true) {
        $answer = Read-Host $question
        $n = 0
        if ([int]::TryParse($answer, [ref] $n) -and $islanders -contains $n) { return $n }
        Write-Host '  Type one of the numbers from the list.'
    }
}

if ($Action -eq 'list') { ShowIslanders; exit 0 }

if (Get-Process -Name 'Tomodachi Life', 'zakuro' -ErrorAction SilentlyContinue) {
    Fail 'Tomodachi Life is running. Close it first (Esc > Quit), or the game will save over this.'
}

if ($Action -eq 'ask') {
    ShowIslanders
    Write-Host 'What should happen?'
    Write-Host '  1  A confession: one islander confesses their feelings to another'
    Write-Host '  2  A proposal: two sweethearts, and one proposes (a wedding follows if it goes well)'
    do { $choice = Read-Host 'Type 1 or 2' } until ($choice -eq '1' -or $choice -eq '2')
    $Action = if ($choice -eq '1') { 'confess' } else { 'propose' }
    $verb = if ($Action -eq 'confess') { 'confess' } else { 'propose' }
    $From = Ask "Who should $verb? (number)"
    $To = Ask "To whom? (number)"
}

if ($islanders -notcontains $From -or $islanders -notcontains $To -or $From -eq $To) {
    Fail 'Pick two different islanders from the list (Romance.ps1 -Action list).'
}
$a = Name $From; $b = Name $To
if ((Female $From) -eq (Female $To)) {
    Fail "$a and $b are both $(if (Female $From) { 'female' } else { 'male' }). The US game only has confessions and proposals between a man and a woman, so it would never show this."
}
$now = Kind $From $To
if (@(7, 8) -contains $now) { Fail "$a and $b are $(KindName $now); the game won't let them be a couple." }
foreach ($pair in @(@($From, $To), @($To, $From))) {
    $o = OtherPartner $pair[0] $pair[1]
    if ($o -ge 0) {
        Fail "$(Name $pair[0]) is $(KindName (Kind $pair[0] $o)) with $(Name $o). The game only sets up a romance between two single islanders, so they'd need to break up first."
    }
}

# keep a copy of the save as it was
$backups = Join-Path (Split-Path $User) 'save backups'
$stamp = Get-Date -Format 'yyyy-MM-dd_HH-mm-ss'
New-Item -ItemType Directory -Force (Join-Path $backups $stamp) | Out-Null
Copy-Item $save (Join-Path $backups "$stamp\savedataArc.txt")

# the game's clock: this computer's, plus how far Esc > Clock set it on
$ahead = 0
$settings = Join-Path (Split-Path $User) 'settings.toml'
if (Test-Path $settings) {
    $line = Select-String -Path $settings -Pattern '^\s*clock_offset\s*=\s*(-?\d+)' | Select-Object -First 1
    if ($line) { $ahead = [int64] $line.Matches[0].Groups[1].Value }
}
$clock = [int64] ((Get-Date) - (Get-Date -Year 2000 -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0 -Millisecond 0)).TotalSeconds + $ahead
$saved = [int64] [BitConverter]::ToUInt32($data, 0x10)
$problem = if ($Action -eq 'confess') { 0x14C } else { 0x154 }
if ($Action -eq 'confess') {
    # friends who like each other a lot, so the answer can be yes
    SetKind $From $To 1
} else {
    SetKind $From $To 2
}
SetFeeling $From $To 200
SetFeeling $To $From 200
# the problem, lasting two days from now (or from when the game last saved,
# if its clock was later)
$r = Rel $From
U32 ($r + 0xC8) ([uint32] ([Math]::Max($clock, $saved) + 2 * 86400))
U32 ($r + 0xCC) 0
U16 ($r + 0xD0) $problem
$data[$r + 0xD2] = 3
U16 ($r + 0xD4) $To
U16 ($r + 0xD6) 0xFFFF
[IO.File]::WriteAllBytes($save, $data)

Write-Host ''
if ($Action -eq 'confess') {
    Write-Host "Done: $a now has a crush on $b, and $b likes $a back."
} else {
    Write-Host "Done: $a and $b are sweethearts, and $a wants to propose."
}
Write-Host "Start the game and open the apartments: $a's window shows a problem. Tap it and help."
Write-Host "The save as it was is in 'save backups\$stamp'."
