@echo off
echo ================================
echo Building Vessel File Search V2
echo ================================

cd /d C:\Dev\v2\apps\desktop

echo.
echo Building frontend + Tauri app...
npm run tauri build

echo.
echo Build complete!
pause