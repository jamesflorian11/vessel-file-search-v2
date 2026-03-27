@echo off
echo =================================
echo Vessel File Search V2 - BUILD
echo =================================

cd /d C:\Dev\v2\apps\desktop

echo.
echo Installing dependencies...
call npm install

echo.
echo Building frontend...
call npm run build

if errorlevel 1 (
    echo.
    echo ❌ Frontend build failed
    pause
    exit /b
)

echo.
echo Building Tauri EXE installer...
call npm run tauri build

if errorlevel 1 (
    echo.
    echo ❌ Tauri build failed
    pause
    exit /b
)

echo.
echo ✅ BUILD COMPLETE
echo.
echo Output location:
echo C:\Dev\v2\apps\desktop\src-tauri\target\release\bundle\nsis\
echo.

pause