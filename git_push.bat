@echo off
title V2 Git Push

echo ================================
echo Vessel File Search V2 - Git Push
echo ================================

cd /d C:\Dev\v2

echo.
echo Checking status...
git status

echo.
echo Adding changes...
git add .

echo.
set /p msg="Enter commit message (or press Enter for auto): "

if "%msg%"=="" (
    for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyy-MM-dd_HH-mm-ss"') do set msg=Update %%i
)

echo.
echo Committing...
git commit -m "%msg%"
if errorlevel 1 (
    echo.
    echo Nothing to commit or commit failed.
    pause
    exit /b
)

echo.
echo Pushing to GitHub...
git push origin main
if errorlevel 1 (
    echo.
    echo Push failed.
    pause
    exit /b
)

echo.
echo ================================
echo Push complete!
echo ================================
pause