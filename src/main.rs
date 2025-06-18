// Windows下隐藏控制台窗口
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// 服务器信息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Server {
    name: String,
    ip: String,
    port: u16,
    status: ServerStatus,
    url: String,
}

// 服务器状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum ServerStatus {
    Unchecked,
    Online,
    Offline,
    Error(u16), // HTTP状态码
}

impl ServerStatus {
    fn to_string(&self) -> String {
        match self {
            ServerStatus::Unchecked => "未检查".to_string(),
            ServerStatus::Online => "✅ 在线".to_string(),
            ServerStatus::Offline => "❌ 离线".to_string(),
            ServerStatus::Error(code) => format!("⚠ 错误 ({})", code),
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ServerStatus::Online => egui::Color32::from_rgb(0, 150, 0),
            ServerStatus::Offline => egui::Color32::from_rgb(200, 0, 0),
            ServerStatus::Error(_) => egui::Color32::from_rgb(255, 165, 0),
            ServerStatus::Unchecked => egui::Color32::GRAY,
        }
    }
}

// 应用程序状态
struct ServerMonitorApp {
    servers: Arc<Mutex<Vec<Server>>>,
    last_check: Instant,
    auto_check_enabled: bool,
    check_interval: Duration,
    // 添加服务器对话框状态
    show_add_dialog: bool,
    new_server_name: String,
    new_server_ip: String,
    new_server_port: String,
    // 删除服务器状态
    selected_server_index: Option<usize>,
    // HTTP客户端
    client: reqwest::Client,
}

impl Default for ServerMonitorApp {
    fn default() -> Self {
        let mut app = Self {
            servers: Arc::new(Mutex::new(Vec::new())),
            last_check: Instant::now(),
            auto_check_enabled: true,
            check_interval: Duration::from_secs(30),
            show_add_dialog: false,
            new_server_name: String::new(),
            new_server_ip: String::new(),
            new_server_port: String::new(),
            selected_server_index: None,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        };

        // 尝试加载配置文件，如果失败则使用默认配置
        if let Err(_) = app.load_servers() {
            app.load_default_servers();
        }

        app
    }
}

impl ServerMonitorApp {
    // 获取可执行文件所在目录
    fn get_exe_dir() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.to_path_buf();
            }
        }
        // 如果获取失败，使用当前工作目录
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    // 获取配置文件路径
    fn get_config_path() -> PathBuf {
        Self::get_exe_dir().join("servers.json")
    }

    // 加载默认服务器配置
    fn load_default_servers(&mut self) {
        let default_ip = "143.86.170.164";
        let default_ports = [8025, 8081, 8000, 3000, 8061, 8080, 8086, 8082, 11434];

        let mut servers = self.servers.lock().unwrap();
        servers.clear();

        for port in default_ports {
            servers.push(Server {
                name: format!("服务器-{}", port),
                ip: default_ip.to_string(),
                port,
                status: ServerStatus::Unchecked,
                url: format!("http://{}:{}", default_ip, port),
            });
        }

        println!("使用默认服务器配置");
    }

    // 保存服务器配置到文件
    fn save_servers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        let servers = self.servers.lock().unwrap();
        let json = serde_json::to_string_pretty(&*servers)?;
        std::fs::write(&config_path, json)?;
        println!("配置已保存到 {:?}", config_path);
        Ok(())
    }

    // 从文件加载服务器配置
    fn load_servers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        let content = std::fs::read_to_string(&config_path)?;
        let loaded_servers: Vec<Server> = serde_json::from_str(&content)?;

        let mut servers = self.servers.lock().unwrap();
        *servers = loaded_servers;

        println!("成功加载配置文件 {:?}", config_path);
        Ok(())
    }

    // 检查单个服务器状态
    async fn check_server_status(&self, server: &mut Server) {
        let response = self.client.get(&server.url).send().await;

        server.status = match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    ServerStatus::Online
                } else {
                    ServerStatus::Error(resp.status().as_u16())
                }
            }
            Err(_) => ServerStatus::Offline,
        };
    }

    // 检查所有服务器状态
    fn check_all_servers(&self) {
        let servers = Arc::clone(&self.servers);
        let client = self.client.clone();

        tokio::spawn(async move {
            let servers_to_check: Vec<Server> = {
                let servers_guard = servers.lock().unwrap();
                servers_guard.clone()
            };

            let mut futures = Vec::new();

            for mut server in servers_to_check {
                let client_clone = client.clone();

                let future = async move {
                    let response = client_clone.get(&server.url).send().await;
                    server.status = match response {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                ServerStatus::Online
                            } else {
                                ServerStatus::Error(resp.status().as_u16())
                            }
                        }
                        Err(_) => ServerStatus::Offline,
                    };
                    server
                };

                futures.push(future);
            }

            // 并发执行所有检查
            let results = futures::future::join_all(futures).await;

            // 更新结果
            let mut servers_guard = servers.lock().unwrap();
            for (i, result) in results.into_iter().enumerate() {
                if i < servers_guard.len() {
                    servers_guard[i].status = result.status;
                }
            }
        });
    }

    // 添加新服务器
    fn add_server(&mut self) {
        if !self.new_server_name.is_empty() && !self.new_server_ip.is_empty() {
            if let Ok(port) = self.new_server_port.parse::<u16>() {
                let server = Server {
                    name: self.new_server_name.clone(),
                    ip: self.new_server_ip.clone(),
                    port,
                    status: ServerStatus::Unchecked,
                    url: format!("http://{}:{}", self.new_server_ip, port),
                };

                self.servers.lock().unwrap().push(server);

                // 清空输入框
                self.new_server_name.clear();
                self.new_server_ip.clear();
                self.new_server_port.clear();
                self.show_add_dialog = false;
            }
        }
    }

    // 删除服务器
    fn remove_server(&mut self, index: usize) {
        let mut servers = self.servers.lock().unwrap();
        if index < servers.len() {
            servers.remove(index);
        }
    }

    // 获取统计信息
    fn get_stats(&self) -> (usize, usize, usize) {
        let servers = self.servers.lock().unwrap();
        let total = servers.len();
        let online = servers
            .iter()
            .filter(|s| s.status == ServerStatus::Online)
            .count();
        let offline = total - online;
        (total, online, offline)
    }
}

impl eframe::App for ServerMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 自动检查逻辑
        if self.auto_check_enabled && self.last_check.elapsed() >= self.check_interval {
            self.check_all_servers();
            self.last_check = Instant::now();
        }

        // 主窗口
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🖥 服务器状态监控");
            ui.separator();

            // 统计信息
            let (total, online, offline) = self.get_stats();
            ui.horizontal(|ui| {
                ui.label(format!("总计: {} 台服务器", total));
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(0, 150, 0),
                    format!("在线: {} 台", online),
                );
                ui.separator();
                ui.colored_label(
                    if offline == 0 {
                        egui::Color32::from_rgb(100, 100, 100) // 黑灰色
                    } else {
                        egui::Color32::from_rgb(200, 0, 0) // 红色
                    },
                    format!("离线: {} 台", offline),
                );
            });

            ui.separator();

            // 控制按钮
            ui.horizontal(|ui| {
                if ui.button("🔄 立即检查").clicked() {
                    self.check_all_servers();
                    self.last_check = Instant::now();
                }

                if ui.button("➕ 添加服务器").clicked() {
                    self.show_add_dialog = true;
                }

                if ui.button("💾 保存配置").clicked() {
                    if let Err(e) = self.save_servers() {
                        eprintln!("保存配置失败: {}", e);
                    }
                }

                if ui.button("📁 加载配置").clicked() {
                    if let Err(e) = self.load_servers() {
                        eprintln!("加载配置失败: {}", e);
                    }
                }

                ui.checkbox(&mut self.auto_check_enabled, "自动检查 (30秒)");
            });

            ui.separator();

            // 服务器列表
            egui::ScrollArea::vertical().show(ui, |ui| {
                let servers = self.servers.lock().unwrap();

                for (i, server) in servers.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(&server.name);
                                ui.label(&server.url);
                                ui.colored_label(server.status.color(), server.status.to_string());
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🗑 删除").clicked() {
                                        self.selected_server_index = Some(i);
                                    }
                                    // 淡蓝色主题的打开按钮
                                    let open_button = egui::Button::new("🌐 打开")
                                        .fill(egui::Color32::from_rgb(173, 216, 230)); // 淡蓝色背景
                                    if ui.add(open_button).clicked() {
                                        if let Err(e) = webbrowser::open(&server.url) {
                                            eprintln!("无法打开浏览器: {}", e);
                                        }
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        });

        // 添加服务器对话框
        if self.show_add_dialog {
            egui::Window::new("添加服务器")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("服务器名称:");
                    ui.text_edit_singleline(&mut self.new_server_name);

                    ui.label("IP地址:");
                    ui.text_edit_singleline(&mut self.new_server_ip);

                    ui.label("端口号:");
                    ui.text_edit_singleline(&mut self.new_server_port);

                    ui.horizontal(|ui| {
                        if ui.button("添加").clicked() {
                            self.add_server();
                        }

                        if ui.button("取消").clicked() {
                            self.show_add_dialog = false;
                            self.new_server_name.clear();
                            self.new_server_ip.clear();
                            self.new_server_port.clear();
                        }
                    });
                });
        }

        // 处理删除服务器
        if let Some(index) = self.selected_server_index.take() {
            self.remove_server(index);
        }

        // 请求重绘以保持UI响应
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

// 初始化中文字体支持
fn init_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 定义不同操作系统的中文字体路径
    let font_paths = if cfg!(target_os = "windows") {
        vec![
            "C:\\Windows\\Fonts\\msyh.ttc",   // 微软雅黑
            "C:\\Windows\\Fonts\\simhei.ttf", // 黑体
            "C:\\Windows\\Fonts\\simsun.ttc", // 宋体
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts/PingFang.ttc",         // 苹方
            "/System/Library/Fonts/Hiragino Sans GB.ttc", // 冬青黑体
            "/System/Library/Fonts/STHeiti Light.ttc",    // 华文黑体
        ]
    } else {
        // Linux
        vec![
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ]
    };

    // 尝试加载中文字体
    let mut font_loaded = false;
    for font_path in font_paths {
        if Path::new(font_path).exists() {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    "chinese_font".to_owned(),
                    egui::FontData::from_owned(font_data),
                );

                // 将中文字体添加到字体族中
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "chinese_font".to_owned());

                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("chinese_font".to_owned());

                font_loaded = true;
                println!("找到中文字体: {}", font_path);
                break;
            }
        }
    }

    if !font_loaded {
        println!("警告: 未找到中文字体，中文可能显示为方块");
    }

    ctx.set_fonts(fonts);
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // 设置日志
    env_logger::init();

    // 加载图标
    let icon_data = include_bytes!("../Icon.png");
    let icon = eframe::icon_data::from_png_bytes(icon_data).unwrap_or_else(|err| {
        eprintln!("加载图标失败: {}", err);
        egui::IconData::default()
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([490.0, 650.0])
            .with_title("服务器状态监控 - Rust版")
            .with_resizable(true)
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "服务器状态监控",
        options,
        Box::new(|cc| {
            // 初始化中文字体
            init_chinese_font(&cc.egui_ctx);
            Ok(Box::new(ServerMonitorApp::default()))
        }),
    )
}
