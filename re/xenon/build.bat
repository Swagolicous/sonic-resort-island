@echo off
rem Builds XenonRecomp (PPC -> C++ recompiler) from refs\UnleashedRecomp\tools\XenonRecomp.
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
set LLVM=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin
set CMAKE=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe
set NINJA=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
"%CMAKE%" -S D:\UnleashedRecomp-Windows\refs\UnleashedRecomp\tools\XenonRecomp -B D:\UnleashedRecomp-Windows\unleashed-rs\re\xenon\build -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER="%LLVM%\clang-cl.exe" -DCMAKE_CXX_COMPILER="%LLVM%\clang-cl.exe" || exit /b 1
"%CMAKE%" --build D:\UnleashedRecomp-Windows\unleashed-rs\re\xenon\build --target XenonRecomp || exit /b 1
