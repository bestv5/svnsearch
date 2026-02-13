@echo off
echo ==============================================
echo   SVN Search - 一键构建工具
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

echo [1/3] 检查 Maven...
where mvn >nul 2>&1
if errorlevel 1 (
    echo 错误: 未找到 Maven，请先安装 Maven
    echo 下载地址: https://maven.apache.org/download.cgi
    pause
    exit /b 1
)

echo [2/3] 编译打包项目...
call mvn clean package -DskipTests

if errorlevel 1 (
    echo 编译失败！
    pause
    exit /b 1
)

echo.
echo ==============================================
echo 构建完成！
echo JAR 文件位置: target\svnsearch.jar
echo ==============================================
echo.
echo 运行命令: java -jar target\svnsearch.jar
echo 访问地址: http://localhost:5000
echo.
pause
