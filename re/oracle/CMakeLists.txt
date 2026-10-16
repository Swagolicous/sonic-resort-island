# Xenon oracle DLL (see oracle.cpp). Built by build.bat with clang-cl, using the same
# float settings as UnleashedRecomp (-march=sandybridge: AVX, no FMA; /fp:strict) so
# its results match the recompiled game bit for bit.
cmake_minimum_required(VERSION 3.20)
project(xenon_oracle C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")

set(RS ${CMAKE_CURRENT_SOURCE_DIR}/../..)
set(XENON_RECOMP ${RS}/../refs/UnleashedRecomp/tools/XenonRecomp)
set(XENON_BUILD ${RS}/re/xenon/build)
set(PPC ${RS}/re/xenon/ppc)

file(GLOB PPC_SOURCES ${PPC}/ppc_recomp.*.cpp)

add_library(xenon_oracle SHARED oracle.cpp stubs.cpp ${PPC}/ppc_func_mapping.cpp ${PPC_SOURCES})
target_include_directories(xenon_oracle PRIVATE ${PPC} ${XENON_RECOMP}/XenonUtils ${XENON_RECOMP}/thirdparty/simde)
target_compile_definitions(xenon_oracle PRIVATE NOMINMAX)
target_compile_options(xenon_oracle PRIVATE -march=sandybridge /fp:strict -Wno-unused-label -Wno-unused-variable)
target_link_libraries(xenon_oracle PRIVATE ${XENON_BUILD}/XenonUtils/XenonUtils.lib ${XENON_BUILD}/thirdparty/disasm/disasm.lib)
