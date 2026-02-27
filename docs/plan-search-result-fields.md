# 搜索结果显示：文件名、路径、大小、SVN 修改时间 — 实现计划

## 现状

- **后端**：`fetch_svn_files` 使用 `svn ls -R`，仅返回**路径字符串列表** `Vec<String>`，无大小、无修改时间。
- **前端**：结果列表展示**文件名**（从路径解析）、**路径**；点击/右键复制路径。

## 目标

搜索结果每条展示：

| 字段     | 说明           | 数据来源              |
|----------|----------------|-----------------------|
| 文件名   | 当前已有       | 从 path 取最后一段    |
| 路径     | 当前已有       | SVN 中的相对路径      |
| 大小     | 新增           | SVN 中文件 size       |
| 修改时间 | 新增           | SVN 中 commit 的 date |

---

## 一、后端改动（Rust）

### 1.1 使用带元数据的 SVN 命令

- 将 **`svn ls -R`** 改为 **`svn list -R --xml`**。
- 认证参数保持不变（`--username` / `--password` 等）。
- XML 输出中包含：`<entry kind="file">`、`<name>`、`<size>`、`<commit><date>`。

### 1.2 依赖

- 在 **`src-tauri/Cargo.toml`** 中增加 XML 解析库，例如：
  - **`quick-xml`**（轻量，可配合 `serde` 或手动解析）。

### 1.3 数据结构

- 定义可序列化结构体（供前端使用），例如：

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SvnFileEntry {
    /// 相对路径（与当前 svn ls 行为一致）
    pub path: String,
    /// 文件大小（字节），仅文件有
    pub size: Option<u64>,
    /// SVN 最后提交时间，如 "2008-11-04T21:57:26.334829Z"
    pub last_modified: Option<String>,
}
```

- **`fetch_svn_files`** 返回类型改为 **`Result<Vec<SvnFileEntry>, String>`**。

### 1.4 XML 解析逻辑

- **`svn list -R --xml`** 输出结构概要：
  - 根节点 `<lists>`，下有多个 `<list path="...">`。
  - 每个 `<list>` 内有多个 `<entry kind="file"|"dir">`。
  - 只处理 **`kind="file"`** 的 entry。
  - 每个 entry：
    - `<name>`：相对当前 list 的路径段。
    - `<size>`：文件大小（目录无）。
    - `<commit><date>`：最后提交时间。
  - **完整 path** = `<list path>` + `/` + `<name>`（path 为空时表示仓库根）。

- 实现新函数，例如 **`parse_svn_xml_output(stdout: String) -> Result<Vec<SvnFileEntry>, String>`**：
  - 解析 `<lists>` / `<list>` / `<entry>`。
  - 只收集 file 类型，拼接 path，读取 size、date，构造 `SvnFileEntry`。
  - 认证失败重试逻辑不变，仅把解析从 `parse_svn_output` 改为 `parse_svn_xml_output`。

### 1.5 兼容与错误

- 若某条 entry 缺少 size 或 date，用 `None`；前端显示为 “-” 或占位。
- XML 解析失败时返回可读错误信息（例如 "解析 SVN 输出失败"）。

---

## 二、前端改动（Vue）

### 2.1 数据类型

- **`allFiles`** / **`filteredFiles`** 从 **`string[]`** 改为 **对象数组**，例如：
  - `{ path: string, size?: number, last_modified?: string }`  
  与后端 `SvnFileEntry` 字段一致（path 必填，size、last_modified 可选）。

### 2.2 搜索与复制

- **搜索**：继续按路径匹配，即 `file.path.toLowerCase().includes(query)`（或当前使用的匹配方式）。
- **复制路径**：点击或右键“复制文件路径”时，复制 **`file.path`**（若当前是对象则传 `item.path`，保持与现有 `copyPath(file)` 语义一致）。

### 2.3 列表展示

- 每条结果展示四列/四行信息（布局可沿用现有列表，仅增加两行或两列）：
  1. **文件名**：`getFileName(file.path)`（或 `file.path` 最后一段）。
  2. **路径**：`file.path`。
  3. **大小**：`formatSize(file.size)`；无 `file.size` 时显示 "-"。
  4. **修改时间**：`formatDate(file.last_modified)`；无则显示 "-"。

- 工具函数：
  - **`formatSize(bytes)`**：将字节转为 "B" / "KB" / "MB" / "GB" 等可读字符串（若项目中已有可复用）。
  - **`formatDate(isoString)`**：将 ISO 8601 的 `last_modified` 转为本地化日期时间字符串（如 "YYYY-MM-DD HH:mm"）。

### 2.4 兼容旧数据（可选）

- 若后端暂时仍可能返回 **字符串数组**（例如过渡期），可在前端做一层兼容：若元素为 string，则转为 `{ path: string, size: undefined, last_modified: undefined }`，这样列表仍能显示路径和文件名，大小/时间为 "-"。

---

## 三、实现顺序建议

1. **后端**  
   - 加依赖 → 定义 `SvnFileEntry` → 改 `fetch_svn_files` 使用 `svn list -R --xml` → 实现 `parse_svn_xml_output` → 返回 `Vec<SvnFileEntry>`。
2. **前端**  
   - 将 `allFiles`/`filteredFiles` 改为对象数组，并适配 `invoke('fetch_svn_files')` 的返回值。  
   - 增加 `formatSize`、`formatDate`，在结果项中展示文件名、路径、大小、修改时间。  
   - 确保搜索、复制路径、右键菜单仍使用 `file.path`。
3. **联调与边界**  
   - 无 size/date 的条目显示 "-"；大仓库下 XML 体积与解析性能可后续再优化（如流式解析）。

---

## 四、小结

| 模块   | 改动要点 |
|--------|-----------|
| 后端   | `svn list -R --xml` + XML 解析 → 返回 `Vec<SvnFileEntry>`（path, size?, last_modified?） |
| 前端   | 结果项类型改为对象；列表展示文件名、路径、大小、修改时间；搜索/复制仍基于 path |

按上述步骤即可在搜索列表中稳定展示**文件名、路径、大小、SVN 修改时间**。
