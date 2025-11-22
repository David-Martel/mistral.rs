@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
nvcc -arch=sm_89 -ptx test_nvcc.cu -o test_nvcc.ptx
echo Exit code: %ERRORLEVEL%
dir test_nvcc.ptx
