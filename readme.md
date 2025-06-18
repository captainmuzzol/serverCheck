# 服务器状态监控工具

一个基于 Rust 和 egui 开发的简易跨平台服务器状态监控工具，提供实时监控、状态统计和便捷管理功能,主要功能由Claude-4模型实现.

## 界面预览

<img src=".//view.png" alt="view" style="zoom:30%;" />

## 功能特性

- 🖥 **实时监控**：定时检查服务器状态，支持HTTP/HTTPS协议
- 📊 **状态统计**：显示总计、在线、离线服务器数量统计
- 🌐 **一键访问**：点击打开按钮直接在浏览器中访问服务器
- 🗑 **服务器管理**：支持添加和删除服务器
- 💾 **配置持久化**：自动保存服务器配置到本地文件
- 🎨 **友好界面**：现代化GUI界面，支持中文显示
- 🔄 **手动刷新**：支持手动触发状态检查
- 🎯 **防误操作**：打开按钮采用淡蓝色主题，与删除按钮区分

## 系统要求

- **Windows**: Windows 7 及以上版本
- **macOS**: macOS 10.12 及以上版本
- **Linux**: 支持现代 Linux 发行版

## 编译说明

### 前置要求

确保已安装 Rust 工具链：
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### macOS 编译
```sh
cargo build --release
```

### Windows 交叉编译（在 macOS/Linux 上）
```sh
# 安装 Windows 目标
rustup target add x86_64-pc-windows-gnu

# 编译
cargo build --release --target x86_64-pc-windows-gnu
```

### Linux 编译
```sh
cargo build --release
```

## 使用方法

1. **启动程序**：双击可执行文件启动应用
2. **添加服务器**：点击"➕ 添加服务器"按钮，输入服务器名称和URL
3. **监控状态**：程序会自动定时检查所有服务器状态
4. **手动刷新**：点击"🔄 立即检查"按钮手动刷新状态
5. **访问服务器**：点击"🌐 打开"按钮在浏览器中打开服务器
6. **删除服务器**：点击"🗑 删除"按钮移除不需要的服务器

## 配置文件

程序会在可执行文件同目录下创建 `servers.json` 配置文件，用于保存服务器列表。配置文件格式：

```json
[
  {
    "name": "服务器名称",
    "url": "https://example.com",
    "status": "Online"
  }
]
```

## 状态说明

- ✅ **在线**：服务器响应正常（HTTP 200）
- ❌ **离线**：服务器无法访问或连接超时
- ⚠ **错误**：服务器返回错误状态码

## 技术栈

- **语言**：Rust
- **GUI框架**：egui + eframe
- **HTTP客户端**：reqwest
- **异步运行时**：tokio
- **序列化**：serde + serde_json
- **浏览器打开**：webbrowser
- **Windows资源**：embed-resource

## 项目结构

```
src/
├── main.rs          # 主程序文件
build.rs             # 构建脚本（Windows资源处理）
resources.rc         # Windows资源文件
Icon.ico            # 应用程序图标
Cargo.toml          # 项目配置
```

## 开发说明

### 添加新功能

1. 在 `ServerMonitorApp` 结构体中添加必要的字段
2. 在 `update` 方法中实现UI逻辑
3. 在相应的事件处理中添加功能代码

### 自定义样式

程序使用 egui 的样式系统，可以通过修改颜色、字体等来自定义界面外观。

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件



## 更新日志

### v1.0.0
- 初始版本发布
- 支持服务器状态监控
- 支持添加/删除服务器
- 支持一键打开浏览器
- 支持Windows图标嵌入
- 支持配置文件持久化
