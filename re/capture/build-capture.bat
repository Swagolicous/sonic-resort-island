@echo off
rem Stages and builds the telemetry capture build of UnleashedRecomp (see README.md).
set PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
set PATH=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin;C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%
rem vcpkg pins an msys2-runtime the mirrors no longer serve, which breaks its own
rem pkgconf bootstrap. Use the native mingw-w64 pkgconf package it would have used
rem (unpacked into tools\ from mirror.msys2.org/mingw/mingw64/mingw-w64-x86_64-pkgconf-1~2.3.0-1).
set PKG_CONFIG=%~dp0tools\mingw64\bin\pkgconf.exe
set VCPKG_KEEP_ENV_VARS=PKG_CONFIG
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Prepare-ReferenceCapture.ps1" %*
