@echo off
echo ==============================================
echo   SVN Search 启动中...
echo ==============================================
echo.

cd /d "%~dp0"

java -jar svnsearch.jar

pause
