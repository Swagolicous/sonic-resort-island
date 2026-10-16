@echo off
rem Incremental rebuild of the already staged capture build (after editing files in
rem _work; regenerate unleashed-reference-trace.patch from there, see README.md).
set PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
set PATH=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin;C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%
ninja -C "%~dp0_work\UnleashedRecomp-trace\out\build\x64-Clang-RelWithDebInfo" UnleashedRecomp
