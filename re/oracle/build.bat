@echo off
rem Builds xenon_oracle.dll (see oracle.cpp) from re\xenon\ppc. Needs re\xenon\build.bat first.
set PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
set LLVM=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin
set CMAKE=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe
set NINJA=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
python "%~dp0gen_stubs.py" || exit /b 1
"%CMAKE%" -S "%~dp0." -B "%~dp0build" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER="%LLVM%\clang-cl.exe" -DCMAKE_CXX_COMPILER="%LLVM%\clang-cl.exe" || exit /b 1
"%CMAKE%" --build "%~dp0build" || exit /b 1
