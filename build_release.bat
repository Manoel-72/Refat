@echo off
echo ========================================
echo RS2 Professional Build Pipeline
echo ========================================

if not exist build mkdir build
if not exist build\windows mkdir build\windows

echo Building project in release mode...
cargo build --release

if errorlevel 1 (
    echo.
    echo [ERROR] Build failed.
    exit /b 1
)

echo Copying executable...
for %%f in (target\release\*.exe) do copy "%%f" "build\windows\" >nul

if exist assets (
    echo Copying assets...
    xcopy assets build\windows\assets /E /I /Y >nul
)

if exist scripts (
    echo Copying scripts...
    xcopy scripts build\windows\scripts /E /I /Y >nul
)

echo.
echo Build completed successfully!
echo Output: build\windows
