@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
set LLVM=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin
set CMAKE=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe
set NINJA=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
"%CMAKE%" -S D:\UnleashedRecomp-Windows\refs\UnleashedRecomp\tools\XenosRecomp -B D:\UnleashedRecomp-Windows\unleashed-rs\re\xenos\build-ninja -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER="%LLVM%\clang-cl.exe" -DCMAKE_CXX_COMPILER="%LLVM%\clang-cl.exe" -DXENOS_RECOMP_DXIL=OFF || exit /b 1
"%CMAKE%" --build D:\UnleashedRecomp-Windows\unleashed-rs\re\xenos\build-ninja --target XenosRecomp || exit /b 1
