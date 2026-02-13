import os
import json
from pathlib import Path

BASE_DIR = Path(__file__).parent
DEFAULT_DATA_DIR = BASE_DIR / 'data'

DEFAULT_CONFIG = {
    'repositories': [],
    'update_interval': 60,
    'svn_path': 'svn' if os.name != 'nt' else 'svn.exe',
    'data_dir': str(DEFAULT_DATA_DIR)
}

# 先使用默认路径加载配置
temp_config_file = DEFAULT_DATA_DIR / 'config.json'

def _load_initial_config():
    DEFAULT_DATA_DIR.mkdir(exist_ok=True)
    if temp_config_file.exists():
        with open(temp_config_file, 'r', encoding='utf-8') as f:
            return json.load(f)
    return DEFAULT_CONFIG.copy()

# 获取实际的数据目录
initial_config = _load_initial_config()
DATA_DIR = Path(initial_config.get('data_dir', str(DEFAULT_DATA_DIR)))
CONFIG_FILE = DATA_DIR / 'config.json'
DB_FILE = DATA_DIR / 'index.db'

def ensure_data_dir():
    DATA_DIR.mkdir(exist_ok=True)

def load_config():
    ensure_data_dir()
    if CONFIG_FILE.exists():
        with open(CONFIG_FILE, 'r', encoding='utf-8') as f:
            return json.load(f)
    return DEFAULT_CONFIG.copy()

def save_config(config):
    # 声明全局变量
    global DATA_DIR, CONFIG_FILE, DB_FILE
    
    ensure_data_dir()
    with open(CONFIG_FILE, 'w', encoding='utf-8') as f:
        json.dump(config, f, indent=2, ensure_ascii=False)
    
    # 更新全局变量
    DATA_DIR = Path(config.get('data_dir', str(DEFAULT_DATA_DIR)))
    CONFIG_FILE = DATA_DIR / 'config.json'
    DB_FILE = DATA_DIR / 'index.db'
    ensure_data_dir()
