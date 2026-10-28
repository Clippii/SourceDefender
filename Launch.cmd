@echo off
cd /d "%~dp0"
if not exist SourceDefender.exe goto build
if not exist SourceDefender.Engine.exe goto build
goto launch
:build
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1"
if errorlevel 1 (pause & exit /b 1)
:launch
if exist SourceDefender.exe start "" "%~dp0SourceDefender.exe"
