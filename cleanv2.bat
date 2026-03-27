@echo off
echo =================================
echo Cleaning V2 build artifacts
echo =================================

cd /d C:\Dev\v2\apps\desktop

echo.
echo Removing frontend dist...
rmdir /s /q dist 2>nul

echo Removing Tauri target...
cd src-tauri
rmdir /s /q target 2>nul

echo.
echo ✅ Clean complete
pause