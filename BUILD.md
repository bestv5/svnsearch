# SVN Search - 构建指南

## 前置要求

### Windows 系统

1. **安装 Java 17+**
   - 下载地址: https://adoptium.net/
   - 或者使用 Microsoft Build of OpenJDK: https://aka.ms/download-jdk

2. **安装 Maven**（或者使用 Gradle Wrapper）
   - 下载地址: https://maven.apache.org/download.cgi

3. **确保 SVN 客户端已安装**（TortoiseSVN）

## 快速开始

### 方式一：使用 Maven

```bash
# 1. 进入项目目录
cd svnsearch

# 2. 编译打包
mvn clean package -DskipTests

# 3. 运行
java -jar target/svnsearch.jar
```

### 方式二：使用 Gradle Wrapper（推荐）

```bash
# 1. 进入项目目录
cd svnsearch

# 2. 编译打包（首次会自动下载依赖）
./gradlew bootJar

# 3. 运行
java -jar build/libs/svnsearch.jar
```

### 方式三：IDEA / Eclipse

1. 用 IDE 打开项目
2. 等待依赖下载完成
3. 运行 SvnSearchApplication

## 访问

服务启动后访问: http://localhost:5000

## 配置

配置文件: `src/main/resources/application.properties`

```properties
server.port=5000
svnsearch.svn-path=svn
svnsearch.update-interval=3600000
svnsearch.data-dir=./data
```

## 打包后分发

编译完成后，会生成 `target/svnsearch.jar`（Maven）或 `build/libs/svnsearch.jar`（Gradle）。

这是一个**单文件 JAR**，包含：
- Java 运行时（需要目标机器安装 Java）
- 所有应用代码
- H2 数据库
- 前端界面

### 分发说明

- JAR 文件需要目标机器安装 **Java 17+**（JRE 即可，无需 JDK）
- 需要系统安装 **TortoiseSVN**（提供 svn.exe 命令）
- 数据目录会在首次运行时自动创建
