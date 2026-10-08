@echo off
rem Stand-in for pkg-config used by scripts/cross-check.ps1 when checking the
rem Linux target from Windows: reports every library (alsa, libudev, ...) as
rem present with no flags. Only good for `cargo check`; linking needs the real
rem libraries, which CI (ubuntu-latest) installs.
echo %* | findstr /C:"--modversion" >nul && (echo 1.0.0& exit /b 0)
echo.
exit /b 0
