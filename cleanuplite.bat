@echo off
echo ================================
echo Cleaning V2 Temp Outputs (Light)
echo ================================

cd /d C:\Dev\v2

echo.
echo Removing frontend dist...
rmdir /s /q apps\desktop\dist 2>nul

echo Removing Tauri app runtime cache (if present)...
rmdir /s /q apps\desktop\src-tauri\gen 2>nul

echo.
echo Light clean complete!
pause