@echo off
rem Makes two islanders want to confess or propose, by changing the save.
rem Close the game first. The save as it was goes into "save backups\".
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "tools\Romance.ps1"
echo.
pause
