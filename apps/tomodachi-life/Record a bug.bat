@echo off
rem Plays the game as usual, but writes down every input so a problem can be
rem replayed exactly. Each run gets its own folder in recordings\, named by
rem the date and time, with a copy of the save as it was when the run started
rem (a replay starts from it), the settings, the log, and notes.txt for what
rem you saw.
cd /d "%~dp0"
for /f %%t in ('powershell -NoProfile -Command "Get-Date -Format yyyy-MM-dd_HH-mm-ss"') do set "STAMP=%%t"
set "DIR=recordings\%STAMP%"
mkdir "%DIR%"
robocopy "user" "%DIR%\user" /E /NFL /NDL /NJH /NJS /NP >nul
copy /y "settings.toml" "%DIR%\settings.toml" >nul
set "ZAKURO_RECORD=%~dp0%DIR%\run.txt"
"Tomodachi Life.exe"
copy /y "zakuro.log" "%DIR%\zakuro.log" >nul
echo What happened, roughly when, and what you expected:> "%DIR%\notes.txt"
start "" notepad "%DIR%\notes.txt"
