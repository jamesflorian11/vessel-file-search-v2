@echo off
echo =================================
echo Vessel File Search V2 - QUICK BUILD
echo =================================

cd /d C:\Dev\v2\apps\desktop

echo.
echo Building frontend...
call npm run build

if errorlevel 1 (
    echo ❌ Frontend build failed
    pause
    exit /b
)

echo.
echo Building Tauri EXE...
call npm run tauri build

if errorlevel 1 (
    echo ❌ Tauri build failed
    pause
    exit /b
)

echo.
echo ✅ QUICK BUILD COMPLETE
pause