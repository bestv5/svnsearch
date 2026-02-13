#!/bin/bash

echo "=============================================="
echo "  SVN Search - macOS 原生应用打包工具"
echo "=============================================="
echo ""

cd "$(dirname "$0")"

# 检查 Java
if ! command -v java &> /dev/null; then
    echo "错误: 未找到 Java，请先安装 Java 17+"
    echo "下载地址: https://adoptium.net/"
    exit 1
fi

# 检查 jpackage
if ! command -v jpackage &> /dev/null; then
    echo "错误: 未找到 jpackage，请安装 JDK 17+ (非 JRE)"
    echo "下载地址: https://adoptium.net/"
    exit 1
fi

echo "[1/4] 清理旧构建..."
rm -rf target build dist

echo "[2/4] 检查 Maven..."
if ! command -v mvn &> /dev/null; then
    echo "错误: 未找到 Maven，请先安装 Maven"
    echo "下载地址: https://maven.apache.org/download.cgi"
    exit 1
fi

echo "[3/4] 编译打包项目..."
mvn clean package -DskipTests

if [ $? -ne 0 ]; then
    echo "编译失败！"
    exit 1
fi

echo "[4/4] 创建 macOS 原生应用..."

# 创建 macOS 应用包
jpackage \
    --input target \
    --main-jar svnsearch-1.0.0.jar \
    --name "SVN Search" \
    --app-version 1.0.0 \
    --vendor "SVNSearch" \
    --description "类似 Everything 的 SVN 文件快速搜索工具" \
    --type dmg \
    --dest dist/ \
    --mac-sign

if [ $? -ne 0 ]; then
    echo "jpackage 失败，尝试其他方式..."
    jpackage \
        --input target \
        --main-jar svnsearch-1.0.0.jar \
        --name "SVN Search" \
        --app-version 1.0.0 \
        --vendor "SVNSearch" \
        --description "类似 Everything 的 SVN 文件快速搜索工具" \
        --type app \
        --dest dist/
fi

echo ""
echo "=============================================="
echo "打包完成！"
echo "输出目录: dist/"
echo "=============================================="
echo ""
echo "文件说明:"
echo "  - SVN Search.app     : macOS 应用程序"
echo "  - 需要系统安装 SVN 客户端"
echo ""
