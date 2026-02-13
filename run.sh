#!/bin/bash
set -e

cd "$(dirname "$0")"

if [ -d "venv" ]; then
    source venv/bin/activate
    echo "Using existing virtual environment"
else
    echo "Creating virtual environment..."
    /usr/local/bin/python3 -m venv venv
    source venv/bin/activate
    echo "Installing dependencies..."
    pip install Flask Flask-CORS APScheduler
fi

echo "Starting SVN Search Server..."
echo ""
echo "=============================================="
echo "  SVN Search - 快速文件搜索工具"
echo "  访问: http://localhost:5000"
echo "=============================================="
echo ""

python app.py
