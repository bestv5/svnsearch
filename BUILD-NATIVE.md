# SVN Search - 原生应用打包指南

## 目标

创建像 IDEA 那样的**原生可执行程序**，无需安装 Java，**解压即用**。

## 前置要求

### Windows

1. **安装 JDK 17+**（必须是 JDK，不是 JRE）
   - 推荐: https://adoptium.net/
   - 或者: Microsoft Build of OpenJDK

2. **安装 Maven**
   - https://maven.apache.org/download.cgi

3. **安装 TortoiseSVN**（提供 svn.exe 命令）

### macOS

1. **安装 JDK 17+**
   - 推荐: https://adoptium.net/
   - 或者: `brew install openjdk@17`

2. **安装 Maven**
   - `brew install maven`

3. **安装 SVN 客户端**
   - `brew install svn`

## 快速开始

### Windows

```bash
# 双击运行
build-native.bat
```

构建完成后会在 `dist` 目录生成：
- `SVN Search.exe` - Windows 原生可执行程序

### macOS

```bash
chmod +x build-native-macos.sh
./build-native-macos.sh
```

构建完成后会在 `dist` 目录生成：
- `SVN Search.dmg` - macOS 安装包
- 或者 `SVN Search.app` - macOS 应用程序

## 工作原理

jpackage 会：

1. **打包 JRE**：将 Java 运行时一起打包
2. **创建启动器**：生成平台原生的启动脚本
3. **生成安装包**：创建 .exe（Windows）或 .dmg（macOS）

## 注意事项

- 打包后的程序**自带 Java 运行时**，无需目标机器安装 Java
- 程序体积会比较大（通常 80-150MB），因为包含完整 JRE
- 如果不想打包 JRE（让用户自己装 Java），可以使用普通 JAR：
  ```bash
  mvn clean package
  java -jar target/svnsearch.jar
  ```

## 故障排除

### jpackage 找不到

确保安装的是 **JDK**，不是 JRE：
```bash
java -version  # 应该显示 "Java HotSpot(TM) 64-Bit Server VM"
```

### 打包失败

可以先尝试普通打包：
```bash
mvn clean package
java -jar target/svnsearch-1.0.0.jar
```
