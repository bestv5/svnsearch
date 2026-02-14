# SVN Search Desktop

基于 Tauri 的 SVN 文件快速搜索工具，类似 Everything。

## 特性

- ✅ **毫秒级搜索**：SQLite 数据库索引
- ✅ **拼音搜索**：支持中文文件名拼音搜索
- ✅ **模糊匹配**：文件名+路径双字段搜索
- ✅ **搜索历史**：自动记录搜索关键词
- ✅ **右键菜单**：复制路径、URL、打开文件夹
- ✅ **设置面板**：自启动、主题、历史数量
- ✅ **托盘图标**：最小化到系统托盘
- ✅ **跨平台**：Windows、macOS、Linux
- ✅ **轻量级**：~10MB 体积

## 技术栈

- **前端**：Vue 3 + Vite
- **后端**：Rust + Tauri
- **数据库**：SQLite
- **搜索**：拼音匹配 + LIKE 查询

## 快速开始

### 前置要求

1. **Node.js 18+**
2. **Rust 1.70+**
3. **系统依赖**：
   - Windows：Visual Studio C++ Build Tools
   - macOS：Xcode Command Line Tools
   - Linux：libwebkit2gtk-4.0-dev

### 安装依赖

```bash
cd desktop
npm install
```

### 开发模式

```bash
npm run tauri:dev
```

### 构建生产版本

```bash
npm run tauri:build
```

## 项目结构

```
desktop/
├── src/                    # Vue 前端
│   ├── App.vue           # 主应用组件
│   └── main.js           # 应用入口
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 入口
│   │   ├── index_engine.rs  # 索引引擎
│   │   ├── database.rs      # SQLite 数据库
│   │   ├── commands.rs      # Tauri API 命令
│   │   ├── svn_client.rs    # SVN 客户端
│   │   ├── tray.rs         # 托盘图标
│   │   ├── autostart.rs    # 自动启动
│   │   └── pinyin.rs       # 拼音搜索
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── package.json
└── vite.config.js
```

## 核心算法

### 索引引擎

- **数据结构**：SQLite 数据库
- **搜索时间**：O(n) 其中 n 是匹配的文件数
- **内存占用**：~100KB/万文件

### 拼音搜索

- **映射表**：常用汉字拼音映射
- **搜索方式**：同时匹配原文和拼音
- **性能**：毫秒级响应

### 搜索流程

1. 用户输入关键词
2. 前端调用 `invoke('search_files', { query })`
3. Rust 后端在数据库中搜索
4. 返回匹配结果（<10ms）

## 使用说明

### 搜索文件

1. 输入关键词开始搜索
2. 点击搜索框查看历史记录
3. 右键点击文件查看快捷操作

### 设置

1. 点击右下角 ⚙️ 图标
2. 配置开机自启动
3. 切换深色/浅色主题
4. 设置搜索历史数量

### 托盘

1. 左键点击托盘图标：显示/隐藏窗口
2. 右键点击托盘图标：显示菜单

## 快捷键

| 快捷键 | 功能 |
|---------|------|
| Ctrl+Space | 聚焦搜索框 |
| Escape | 关闭面板/菜单 |

## 性能优化

- ✅ SQLite 索引加速
- ✅ 分批索引（每批 500 个文件）
- ✅ 搜索防抖（300ms）
- ✅ 虚拟滚动（长列表优化）
- ✅ 懒加载（按需加载）

## 开发计划

- [ ] 添加仓库管理界面
- [ ] 支持增量索引更新
- [ ] 添加文件预览功能
- [ ] 支持正则表达式搜索
- [ ] 添加搜索过滤器

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
