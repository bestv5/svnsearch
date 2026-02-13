#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "创建 SVN Search 便携包..."

# 创建打包目录
PACK_DIR="./svnsearch-portable"
mkdir -p "$PACK_DIR"
mkdir -p "$PACK_DIR/static"

# 复制核心文件
cp -f app.py "$PACK_DIR/"
cp -f config.py "$PACK_DIR/"
cp -f models.py "$PACK_DIR/"
cp -f svn_client.py "$PACK_DIR/"
cp -f index_service.py "$PACK_DIR/"
cp -f static/index.html "$PACK_DIR/static/"
cp -f requirements.txt "$PACK_DIR/"

# 创建启动脚本（Linux/Mac）
cat > "$PACK_DIR/run.sh" << 'EOF'
#!/bin/bash
set -e

cd "$(dirname "$0")"

if [ -d "venv" ]; then
    source venv/bin/activate
    echo "使用现有虚拟环境"
else
    echo "创建虚拟环境..."
    python3 -m venv venv
    source venv/bin/activate
    echo "安装依赖..."
    pip install -r requirements.txt
fi

echo "启动 SVN Search 服务..."
echo "访问: http://localhost:5000"
echo ""

python app.py
EOF

chmod +x "$PACK_DIR/run.sh"

# 创建启动脚本（Windows）
cat > "$PACK_DIR/run.bat" << 'EOF'
@echo off
cd /d "%~dp0"

echo 正在启动 SVN Search 服务...
echo ==============================================

if exist "venv\Scripts\activate.bat" (
    echo 使用现有虚拟环境...
    call venv\Scripts\activate.bat
) else (
    echo 创建虚拟环境...
    python -m venv venv
    call venv\Scripts\activate.bat
    echo 安装依赖...
    pip install -r requirements.txt
)

echo 启动服务...
echo 访问: http://localhost:5000
echo ==============================================
echo.

python app.py
pause
EOF

# 打包成 zip 文件
ZIP_FILE="./svnsearch-portable.zip"
if [ -f "$ZIP_FILE" ]; then
    rm "$ZIP_FILE"
fi

zip -r "$ZIP_FILE" "$PACK_DIR" > /dev/null

# 清理
rm -rf "$PACK_DIR"

echo "打包完成！"
echo "便携包位置: $ZIP_FILE"
echo ""
echo "使用方法:"
echo "1. 解压 svnsearch-portable.zip"
echo "2. 运行 ./run.sh"
echo "3. 访问 http://localhost:5000"
