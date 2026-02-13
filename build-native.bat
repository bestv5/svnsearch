@echo off
setlocal enabledelayedexpansion

echo ==============================================
echo   SVN Search - 原生应用打包工具
echo ==============================================
echo.

cd /d "%~dp0"

:: 检查 Java
java -version >nul 2>&1
if errorlevel 1 (
    echo 错误: 未找到 Java，请先安装 Java 17+
    echo 下载地址: https://adoptium.net/
    pause
    exit /b 1
)

:: 检查 jpackage
where jpackage >nul 2>&1
if errorlevel 1 (
    echo 错误: 未找到 jpackage，请安装 JDK 17+ (非 JRE)
    echo 下载地址: https://adoptium.net/
    pause
    exit /b 1
)

echo [1/4] 清理旧构建...
if exist "target" rmdir /s /q "target"
if exist "build" rmdir /s /q "build"
if exist "dist" rmdir /s /q "dist"

echo [2/4] 检查 Maven...
where mvn >nul 2>&1
if errorlevel 1 (
    echo 错误: 未找到 Maven，请先安装 Maven
    echo 下载地址: https://maven.apache.org/download.cgi
    pause
    exit /b 1
)

echo [3/4] 编译打包项目...
call mvn clean package -DskipTests

if errorlevel 1 (
    echo 编译失败！
    pause
    exit /b 1
)

echo [4/4] 创建原生应用...
set JAR_PATH=target\svnsearch-1.0.0.jar
set APP_NAME=SVN Search
set OUTPUT_DIR=dist

echo 正在创建 Windows 原生应用...
jpackage ^
    --input target ^
    --main-jar svnsearch-1.0.0.jar ^
    --name "%APP_NAME%" ^
    --app-version 1.0.0 ^
    --vendor "SVNSearch" ^
    --description "类似 Everything 的 SVN 文件快速搜索工具" ^
    --icon icon.ico ^
    --type exe ^
    --dest "%OUTPUT_DIR%" ^
    --runtime-image runtime ^
    --win-console

if errorlevel 1 (
    echo jpackage 失败，尝试不使用 runtime-image...
    jpackage ^
        --input target ^
        --main-jar svnsearch-1.0.0.jar ^
        --name "%APP_NAME%" ^
        --app-version 1.0.0 ^
        --vendor "SVNSearch" ^
        --description "类似 Everything 的 SVN 文件快速搜索工具" ^
        --type exe ^
        --dest "%OUTPUT_DIR%" ^
        --win-console
)

echo.
echo ==============================================
echo 打包完成！
echo 输出目录: dist\
echo ==============================================
echo.
echo 文件说明:
echo   - SVN Search.exe     : 直接运行的应用程序
echo   - 需要系统安装 TortoiseSVN
echo.
pause
