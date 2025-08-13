// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

use auto_launch::AutoLaunch;
use eframe::egui::{self, Color32, Stroke, Rounding};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use single_instance::SingleInstance;

// 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    url: String,
    interval_hours: u64,
    enable_scheduler: bool,
    auto_launch_enabled: bool,
    minimize_to_background: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            url: "https://devcloud.greenvoltis.com/maven/settings.xml".to_string(),
            interval_hours: 10,
            enable_scheduler: false,
            auto_launch_enabled: false,
            minimize_to_background: true,
        }
    }
}

// 顶部添加辅助函数
fn get_config_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(".msettings_config.json")
}

fn load_config() -> AppConfig {
    let path = get_config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => {
            match serde_json::from_str::<AppConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning: Failed to parse config file: {}, using defaults", e);
                    AppConfig::default()
                }
            }
        }
        Err(_) => {
            // 配置文件不存在或无法读取，使用默认配置
            AppConfig::default()
        }
    }
}

fn save_config(config: &AppConfig) {
    let path = get_config_file_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Warning: Failed to create config directory: {}", e);
            return;
        }
    }
    match serde_json::to_string_pretty(config) {
        Ok(json) => {
            match fs::File::create(&path) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(json.as_bytes()) {
                        eprintln!("Warning: Failed to write config file: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to create config file: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to serialize config: {}", e);
        }
    }
}

// 创建AutoLaunch实例的辅助函数
fn create_auto_launch(app_name: &str, exe_path: &str) -> Result<AutoLaunch, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let auto_launch = AutoLaunch::new(app_name, exe_path, &[] as &[&str]);
    
    #[cfg(not(target_os = "windows"))]
    let auto_launch = AutoLaunch::new(app_name, exe_path, false, &[] as &[&str]);
    
    Ok(auto_launch)
}

// 保持向后兼容的函数
#[allow(dead_code)]
fn load_saved_url() -> Option<String> {
    Some(load_config().url)
}

fn save_url_to_config(url: &str) {
    let mut config = load_config();
    config.url = url.to_string();
    save_config(&config);
}

fn setup_custom_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    use std::path::Path;

    let mut fonts = FontDefinitions::default();

    // 多平台字体候选路径
    let font_paths = [
        "assets/fonts/SourceHanSerifCN-Regular-1.otf",
        "../Resources/assets/fonts/SourceHanSerifCN-Regular-1.otf",
        "Contents/Resources/assets/fonts/SourceHanSerifCN-Regular-1.otf",
        "/System/Library/Fonts/PingFang.ttc",                    // macOS PingFang
        "/System/Library/Fonts/Hiragino Sans GB.ttc",            // macOS Hiragino
    ];

    let mut font_loaded = false;

    for font_path in &font_paths {
        if Path::new(font_path).exists() {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    "chinese_font".to_string(),
                    FontData::from_owned(font_data),
                );
                font_loaded = true;
                println!("✅ 成功加载字体: {}", font_path);
                break;
            }
        }
    }

    if font_loaded {
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "chinese_font".to_string());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "chinese_font".to_string());
    } else {
        println!("⚠️ 无法加载嵌入字体，尝试系统字体");

        #[cfg(target_os = "macos")]
        let fallback_font = "PingFang SC";

        #[cfg(target_os = "windows")]
        let fallback_font = "Microsoft YaHei";

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let fallback_font = "Noto Sans SC"; // Linux 推荐字体

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, fallback_font.to_string());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, fallback_font.to_string());
    }

    ctx.set_fonts(fonts);
}

#[cfg(target_os = "windows")]
fn get_m2_settings_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap();
    PathBuf::from(home).join(".m2").join("settings.xml")
}

#[cfg(not(target_os = "windows"))]
fn get_m2_settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap();
    PathBuf::from(home).join(".m2").join("settings.xml")
}

struct AppState {
    url: String,
    previous_url: String,  // 新增
    interval_hours: u64,
    previous_interval_hours: u64,  // 新增
    status: String,
    running: bool,
    enable_scheduler: bool,
    history: Vec<String>,
    shared_state: Arc<Mutex<SharedState>>,
    auto_launch_enabled: bool,
    auto_launch: AutoLaunch,

    // 新增字段用于线程管理
    scheduler_running: bool,
    stop_signal: Arc<Mutex<bool>>,
    next_update_time: Option<chrono::DateTime<chrono::Local>>,
    // 窗口显示控制
    show_window: bool,
    minimize_to_background: bool,
    // 重新显示窗口的机制
    should_show_window: Arc<Mutex<bool>>,
}

struct SharedState {
    enable_scheduler: bool,
    url: String,
    interval_hours: u64,
    history: Vec<String>,
}

// 颜色常量
const PRIMARY_COLOR: Color32 = Color32::from_rgb(102, 126, 234);
// const SECONDARY_COLOR: Color32 = Color32::from_rgb(118, 75, 162);
const SUCCESS_COLOR: Color32 = Color32::from_rgb(76, 175, 80);
const ERROR_COLOR: Color32 = Color32::from_rgb(244, 67, 54);
const WARNING_COLOR: Color32 = Color32::from_rgb(255, 152, 0);
const BACKGROUND_COLOR: Color32 = Color32::from_rgb(248, 249, 250);
const CARD_COLOR: Color32 = Color32::from_rgb(255, 255, 255);
const TEXT_COLOR: Color32 = Color32::from_rgb(51, 51, 51);
const SECONDARY_TEXT_COLOR: Color32 = Color32::from_rgb(102, 102, 102);
const BORDER_COLOR: Color32 = Color32::from_rgb(225, 229, 233);

impl Default for AppState {
    fn default() -> Self {
        let app_name = "AutoUpdateMavenSettings";
        
        // 安全地获取可执行文件路径
        let exe_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Warning: Failed to get current exe path: {}, using fallback", e);
                std::path::PathBuf::from("AutoUpdateMavenSettings")
            }
        };
        
        let exe_path_str = match exe_path.to_str() {
            Some(path) => path,
            None => {
                eprintln!("Warning: Exe path contains invalid UTF-8, using fallback");
                "AutoUpdateMavenSettings"
            }
        };
        
        // 不同平台的AutoLaunch API有不同的参数
        let auto_launch = match create_auto_launch(app_name, exe_path_str) {
            Ok(launcher) => launcher,
            Err(e) => {
                eprintln!("Warning: Failed to create AutoLaunch: {}", e);
                // 创建一个空的AutoLaunch作为fallback
                #[cfg(target_os = "windows")]
                let fallback = AutoLaunch::new("fallback", "fallback", &[] as &[&str]);
                #[cfg(not(target_os = "windows"))]
                let fallback = AutoLaunch::new("fallback", "fallback", false, &[] as &[&str]);
                fallback
            }
        };

        // 读取配置文件
        eprintln!("Loading configuration...");
        let config = load_config();
        eprintln!("Configuration loaded successfully");
        
        let shared_state = Arc::new(Mutex::new(SharedState {
            enable_scheduler: config.enable_scheduler,
            url: config.url.clone(),
            interval_hours: config.interval_hours,
            history: Vec::new(),
        }));

        Self {
            url: config.url.clone(),
            previous_url: config.url,
            interval_hours: config.interval_hours,
            previous_interval_hours: config.interval_hours,
            status: "未开始".to_string(),
            running: false,
            enable_scheduler: config.enable_scheduler,
            history: Vec::new(),
            shared_state,
            auto_launch_enabled: config.auto_launch_enabled,
            auto_launch,
            scheduler_running: false,
            stop_signal: Arc::new(Mutex::new(false)),
            next_update_time: None,
            show_window: true,
            minimize_to_background: config.minimize_to_background,
            should_show_window: Arc::new(Mutex::new(false)),
        }
    }
}

impl AppState {
    /// 保存当前所有配置到文件
    fn save_current_config(&self) {
        let config = AppConfig {
            url: self.url.clone(),
            interval_hours: self.interval_hours,
            enable_scheduler: self.enable_scheduler,
            auto_launch_enabled: self.auto_launch_enabled,
            minimize_to_background: self.minimize_to_background,
        };
        save_config(&config);
    }

    /// 处理窗口显示状态
    fn handle_window_visibility(&mut self, ctx: &egui::Context) {
        // 检查文件信号
        if check_show_signal() {
            self.show_window = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        
        // 检查是否需要重新显示窗口
        if let Ok(mut should_show) = self.should_show_window.try_lock() {
            if *should_show {
                self.show_window = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                *should_show = false;
            }
        }
        
        // 添加键盘快捷键支持 - 按ESC最小化到后台
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.minimize_to_background {
            self.show_window = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }
        
        // 添加组合键支持 - Cmd+Shift+M (macOS) 或 Ctrl+Shift+M (Windows) 重新显示窗口
        let show_key_combo = if cfg!(target_os = "macos") {
            ctx.input(|i| i.modifiers.mac_cmd && i.modifiers.shift && i.key_pressed(egui::Key::M))
        } else {
            ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::M))
        };
        
        if show_key_combo {
            self.show_window = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
    /// 异步执行立即更新，避免阻塞 UI 线程
    fn perform_immediate_update_async(&mut self, ctx: &egui::Context) {
        // 校验 URL
        if !self.url.starts_with("http") {
            self.status = "请输入有效的下载地址".to_string();
            return;
        }

        // 设置状态，让用户知道正在更新
        if self.enable_scheduler {
            self.status = "立即更新中，定时任务已启动".to_string();
        } else {
            self.status = "手动更新中".to_string();
        }
        self.running = true;

        // 克隆需要在子线程中使用的值
        let url = self.url.clone();
        let shared_state = Arc::clone(&self.shared_state);
        let ctx_clone = ctx.clone();

        // 后台线程执行下载和替换
        thread::spawn(move || {
            let now = chrono::Local::now();
            let result = download_and_replace(&url);
            let record = match result {
                Ok(_) => format!("{}: 立即更新成功", now.format("%Y-%m-%d %H:%M:%S")),
                Err(e) => format!("{}: 立即更新失败 - {}", now.format("%Y-%m-%d %H:%M:%S"), e),
            };

            // 只更新共享历史记录，不直接修改 AppState
            if let Ok(mut shared) = shared_state.lock() {
                shared.history.push(record);
            }

            // 通知主线程重绘界面
            ctx_clone.request_repaint();
        });
    }

    fn draw_header(&self, ui: &mut egui::Ui, content_width: f32) {
        let header_height = 90.0;

        // 确保头部完全覆盖容器宽度
        ui.allocate_ui_with_layout(
            egui::vec2(content_width, header_height),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                // 绘制头部背景 - 完全覆盖
                let rect = egui::Rect::from_min_size(
                    ui.available_rect_before_wrap().min,
                    egui::vec2(content_width, header_height)
                );
                ui.painter().rect_filled(
                    rect,
                    Rounding { nw: 20.0, ne: 20.0, sw: 0.0, se: 0.0 },
                    PRIMARY_COLOR,
                );

                ui.add_space(10.0);
                // 内容区域保持居中，但给左右留40px边距与下方内容对齐
                ui.allocate_ui_with_layout(
                    egui::vec2(content_width - 80.0, header_height - 20.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        // 标题水平居中并排显示
                        ui.horizontal_centered(|ui| {
                            // 尝试加载 logo - 支持多个路径和工作目录检测
                            let current_dir = std::env::current_dir().unwrap_or_default();
                            let exe_dir = std::env::current_exe()
                                .ok()
                                .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                                .unwrap_or_default();

                            let logo_paths = [
                                // 开发环境路径 - 优先使用指定的图标
                                "assets/icon/mavi_icon_shadow.png",
                                // macOS 应用包内路径
                                "../Resources/assets/icon/mavi_icon_shadow.png", 
                                "Contents/Resources/assets/icon/mavi_icon_shadow.png",
                                // 相对于可执行文件的路径
                                &format!("{}/assets/icon/mavi_icon_shadow.png", current_dir.display()),
                                &format!("{}/Contents/Resources/assets/icon/mavi_icon_shadow.png", exe_dir.display()),
                                &format!("{}/../Resources/assets/icon/mavi_icon_shadow.png", exe_dir.display()),
                                // 备用路径
                                &format!("{}/assets/icon/mavi_icon_shadow.png", exe_dir.display()),
                            ];

                            // 加载logo纹理（使用缓存机制）
                            let mut logo_loaded = false;
                            for logo_path in &logo_paths {
                                if std::path::Path::new(logo_path).exists() {
                                    if let Ok(image_bytes) = std::fs::read(logo_path) {
                                        if let Ok(image) = image::load_from_memory(&image_bytes) {
                                            let rgba_image = image.to_rgba8();
                                            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                                            let pixels = rgba_image.into_raw();
                                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                            let texture_handle = ui.ctx().load_texture("app_logo", color_image, egui::TextureOptions::default());

                                            ui.add(egui::Image::from_texture(&texture_handle).fit_to_exact_size(egui::vec2(72.0, 72.0)));
                                            ui.add_space(12.0);
                                            logo_loaded = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            if !logo_loaded {
                                // 如果无法加载logo，显示一个简单的文字图标
                                ui.allocate_ui_with_layout(
                                    egui::vec2(72.0, 72.0),
                                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new("M")
                                                .size(48.0)
                                                .color(Color32::WHITE)
                                                .strong()
                                        );
                                    }
                                );
                                ui.add_space(12.0);
                                println!("⚠️ Logo 未找到，使用文字替代");
                            }

                            ui.vertical(|ui| {
                                ui.add_space(20.0);
                                // 使用垂直居中对齐的水平布局
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 30.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new("MSettings")
                                                .size(22.0)
                                                .color(Color32::WHITE)
                                                .strong()
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            egui::RichText::new("Maven 配置自动更新工具")
                                                .size(13.0)
                                            .color(Color32::from_rgba_unmultiplied(255, 255, 255, 230))
                                    );
                                });
                            });
                        });
                    }
                );
            }
        );
    }

    fn draw_section_title(&self, ui: &mut egui::Ui, title: &str) {
        ui.horizontal(|ui| {
            // 绘制左侧装饰条
            let rect = ui.allocate_space(egui::vec2(4.0, 16.0)).1;
            ui.painter().rect_filled(
                rect,
                Rounding::same(2.0),
                PRIMARY_COLOR,
            );

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(title)
                    .size(15.0)
                    .color(TEXT_COLOR)
                    .strong()
            );
        });
        ui.add_space(12.0);
    }

    fn draw_custom_switch(&self, ui: &mut egui::Ui, label: &str, value: bool) -> egui::Response {
        let switch_height = 45.0;

        let (id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), switch_height));
        let response = ui.interact(rect, id, egui::Sense::click());

        // 背景卡片
        ui.painter().rect_filled(
            rect,
            Rounding::same(10.0),
            BACKGROUND_COLOR,
        );

        // 标签
        let text_pos = rect.left_center() + egui::vec2(16.0, 0.0);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
            TEXT_COLOR,
        );

        // 开关
        let switch_width = 46.0;
        let switch_height = 26.0;
        let switch_rect = egui::Rect::from_center_size(
            rect.right_center() - egui::vec2(32.0, 0.0),
            egui::vec2(switch_width, switch_height)
        );

        // 绘制开关背景
        let bg_color = if value { PRIMARY_COLOR } else { Color32::from_rgb(221, 221, 221) };
        ui.painter().rect_filled(switch_rect, Rounding::same(13.0), bg_color);

        // 绘制开关按钮
        let circle_radius = 11.0;
        let circle_x = if value {
            switch_rect.max.x - circle_radius - 2.0
        } else {
            switch_rect.min.x + circle_radius + 2.0
        };
        let circle_center = egui::pos2(circle_x, switch_rect.center().y);

        ui.painter().circle_filled(
            circle_center,
            circle_radius,
            Color32::WHITE,
        );

        response
    }

    fn draw_custom_button(&self, ui: &mut egui::Ui, text: &str, primary: bool, enabled: bool) -> egui::Response {
        let button_height = 40.0;

        let (id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), button_height));
        let response = ui.interact(rect, id, if enabled { egui::Sense::click() } else { egui::Sense::hover() });

        let (bg_color, text_color) = if primary {
            if enabled {
                (PRIMARY_COLOR, Color32::WHITE)
            } else {
                (Color32::from_rgb(200, 200, 200), Color32::WHITE)
            }
        } else {
            (BACKGROUND_COLOR, SECONDARY_TEXT_COLOR)
        };

        // 绘制按钮背景
        let rounding = Rounding::same(10.0);
        ui.painter().rect_filled(rect, rounding, bg_color);
        if !primary {
            ui.painter().rect_stroke(rect, rounding, Stroke::new(2.0, BORDER_COLOR));
        }

        // 绘制按钮文本
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(15.0),
            text_color,
        );

        response
    }

    fn draw_status_card(&self, ui: &mut egui::Ui) {
        let card_height = 50.0;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), card_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // 状态卡片背景 - 增强立体效果
                let rect = ui.available_rect_before_wrap();

                // 绘制阴影
                let shadow_rect = rect.translate(egui::vec2(0.0, 2.0));
                ui.painter().rect_filled(
                    shadow_rect,
                    Rounding::same(10.0),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 15),
                );

                // 绘制主背景
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(10.0),
                    Color32::from_rgb(227, 242, 253),
                );

                // 绘制高光边框
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(10.0),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 120))
                );

                ui.add_space(16.0);

                // 状态指示器 - 垂直居中
                ui.allocate_ui_with_layout(
                    egui::vec2(16.0, card_height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add_space((card_height - 10.0) / 2.0);
                        let dot_color = if self.running { SUCCESS_COLOR } else { WARNING_COLOR };
                        let dot_center = ui.cursor().min + egui::vec2(5.0, 5.0);

                        // 绘制状态点阴影
                        ui.painter().circle_filled(dot_center + egui::vec2(1.0, 1.0), 5.0, Color32::from_rgba_unmultiplied(0, 0, 0, 30));
                        // 绘制状态点
                        ui.painter().circle_filled(dot_center, 5.0, dot_color);
                        // 绘制高光
                        ui.painter().circle_filled(dot_center + egui::vec2(-1.0, -1.0), 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150));
                    }
                );

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!("状态：{}", self.status))
                        .size(14.0)
                        .color(TEXT_COLOR)
                        .strong()
                );
            }
        );
    }

    fn draw_next_update_card(&self, ui: &mut egui::Ui) {
        let card_height = 50.0;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), card_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // 下次更新卡片背景 - 增强立体效果
                let rect = ui.available_rect_before_wrap();

                // 绘制阴影
                let shadow_rect = rect.translate(egui::vec2(0.0, 2.0));
                ui.painter().rect_filled(
                    shadow_rect,
                    Rounding::same(10.0),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 15),
                );

                // 绘制主背景
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(10.0),
                    Color32::from_rgb(233, 247, 241), // 淡绿色背景
                );

                // 绘制高光边框
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(10.0),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 120))
                );

                ui.add_space(16.0);

                // 时钟图标
                ui.allocate_ui_with_layout(
                    egui::vec2(16.0, card_height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add_space((card_height - 10.0) / 2.0);
                        ui.label("⏰");
                    }
                );

                ui.add_space(8.0);

                if let Some(next_time) = self.next_update_time {
                    ui.label(
                        egui::RichText::new(format!("下次更新：{}", next_time.format("%m-%d %H:%M")))
                            .size(14.0)
                            .color(Color32::from_rgb(34, 139, 34))
                            .strong()
                    );
                }
            }
        );
    }

    fn draw_history_section(&self, ui: &mut egui::Ui) {
        // 历史记录标题
        ui.horizontal(|ui| {
            ui.label("📋");
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("执行历史记录")
                    .size(14.0)
                    .color(TEXT_COLOR)
                    .strong()
            );
        });
        ui.add_space(12.0);

        // 历史记录容器 - 增强立体效果
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();

                // 绘制内阴影效果
                let inner_shadow_rect = rect.shrink(1.0);
                ui.painter().rect_filled(
                    inner_shadow_rect,
                    Rounding::same(9.0),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 8),
                );

                // 绘制主背景
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(10.0),
                    BACKGROUND_COLOR,
                );

                // 绘制边框
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(10.0),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 30))
                );

                ui.add_space(12.0);

                if self.history.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("暂无执行记录")
                                .size(13.0)
                                .color(SECONDARY_TEXT_COLOR)
                        );
                    });
                } else {
                    for (i, record) in self.history.iter().rev().enumerate() {
                        // 使用垂直居中的布局
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 24.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add_space(12.0);

                                // 状态图标
                                let (icon, color) = if record.contains("成功") {
                                    ("✅", SUCCESS_COLOR)
                                } else {
                                    ("❌", ERROR_COLOR)
                                };

                                ui.label(icon);
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(record)
                                        .size(12.0)
                                        .color(color)
                                );
                            });

                        if i < self.history.len() - 1 {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);
                        } else {
                            ui.add_space(8.0);
                        }
                    }
                }

                ui.add_space(12.0);
            });
    }

    fn draw_left_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // 基础配置
        self.draw_section_title(ui, "基础配置");

        ui.label(
            egui::RichText::new("Settings.xml 下载地址")
                .size(13.0)
                .color(SECONDARY_TEXT_COLOR)
        );
        ui.add_space(6.0);

        // 垂直居中的输入框
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 36.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_sized(
                    [ui.available_width(), 36.0],
                    egui::TextEdit::singleline(&mut self.url)
                        .hint_text("请输入 HTTP 下载链接...")
                        .desired_width(ui.available_width())
                        .vertical_align(egui::Align::Center)
                );
            }
        );

        // 同步 URL 到共享状态并保存配置
        if let Ok(mut shared) = self.shared_state.lock() {
            shared.url = self.url.clone();
        }
        
        // 如果URL发生变化，保存到配置文件
        if self.url != self.previous_url {
            save_url_to_config(&self.url);
            self.previous_url = self.url.clone();
        }

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("自动更新间隔")
                    .size(13.0)
                    .color(SECONDARY_TEXT_COLOR)
            );
        });
        ui.add_space(6.0);

        // 使用垂直居中的布局容器
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 36.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // 数值输入框
                ui.add_sized(
                    [75.0, 36.0],
                    egui::DragValue::new(&mut self.interval_hours)
                        .clamp_range(1..=168)
                );
                ui.add_space(8.0);
                // 垂直居中的"小时"标签
                ui.label(
                    egui::RichText::new("小时")
                        .size(13.0)
                        .color(SECONDARY_TEXT_COLOR)
                );
            });

            // 同步配置到共享状态并保存
            if let Ok(mut shared) = self.shared_state.lock() {
                shared.url = self.url.clone();
                shared.interval_hours = self.interval_hours;
            }
            
            // 如果间隔小时数发生变化，保存配置
            if self.interval_hours != self.previous_interval_hours {
                self.save_current_config();
                self.previous_interval_hours = self.interval_hours;
            }

            // 新增：如果地址发生变化则保存到配置文件
            if self.url != self.previous_url {
                save_url_to_config(&self.url);
                self.previous_url = self.url.clone();
            }

        ui.add_space(20.0);

        // 功能开关
        self.draw_section_title(ui, "功能开关");

        if self.draw_custom_switch(ui, "定时任务", self.enable_scheduler).clicked() {
            self.enable_scheduler = !self.enable_scheduler;
            if let Ok(mut shared) = self.shared_state.lock() {
                shared.enable_scheduler = self.enable_scheduler;
            }
            self.save_current_config();
        }

        ui.add_space(10.0);

        if self.draw_custom_switch(ui, "开机自启", self.auto_launch_enabled).clicked() {
            self.auto_launch_enabled = !self.auto_launch_enabled;
            if self.auto_launch_enabled {
                if let Err(e) = self.auto_launch.enable() {
                    eprintln!("Failed to enable auto launch: {}", e);
                    self.auto_launch_enabled = false;
                }
            } else {
                if let Err(e) = self.auto_launch.disable() {
                    eprintln!("Failed to disable auto launch: {}", e);
                }
            }
            self.save_current_config();
        }

        ui.add_space(10.0);

        if self.draw_custom_switch(ui, "关闭窗口后台运行", self.minimize_to_background).clicked() {
            self.minimize_to_background = !self.minimize_to_background;
            self.save_current_config();
        }
        
        // 添加说明文字
        if self.minimize_to_background {
            ui.add_space(5.0);
            let shortcut = if cfg!(target_os = "macos") {
                "Cmd+Shift+M"
            } else {
                "Ctrl+Shift+M"
            };
            ui.label(
                egui::RichText::new(&format!("提示: 关闭窗口时程序将在后台继续运行\n按ESC键最小化，按{}重新显示，或双击应用图标", shortcut))
                    .size(11.0)
                    .color(SECONDARY_TEXT_COLOR)
            );
        }

        ui.add_space(20.0);

        // 操作按钮
        self.draw_section_title(ui, "操作控制");

        if !self.running {
            if self.draw_custom_button(ui, "启动自动更新", true, true).clicked() {
                if self.url.starts_with("http") && self.interval_hours > 0 {
                    self.start_update_task(ctx);
                } else {
                    self.status = "请输入有效的 URL 和间隔".to_string();
                }
            }
        } else {
            if self.draw_custom_button(ui, "停止自动更新", true, true).clicked() {
                self.stop_scheduler();
            }
        }
    }

    fn draw_right_panel(&self, ui: &mut egui::Ui) {
        // 状态显示
        self.draw_section_title(ui, "运行状态");
        self.draw_status_card(ui);

        ui.add_space(16.0);

        // 下次执行时间显示
        if self.scheduler_running && self.next_update_time.is_some() {
            self.draw_next_update_card(ui);
            ui.add_space(16.0);
        }

        // 执行历史
        self.draw_history_section(ui);
    }

    fn start_update_task(&mut self, ctx: &egui::Context) {
        // 立即执行一次更新：改用异步版本
        self.perform_immediate_update_async(ctx);

        if self.enable_scheduler && !self.scheduler_running {
            // 设置下次更新时间
            let now = chrono::Local::now();
            self.next_update_time = Some(now + chrono::Duration::hours(self.interval_hours as i64));

            // 启动定时任务
            self.scheduler_running = true;
            self.running = true;
            self.status = "定时任务已启动".to_string();

            // 重置停止信号
            if let Ok(mut stop) = self.stop_signal.lock() {
                *stop = false;
            }

            let shared_state = Arc::clone(&self.shared_state);
            let stop_signal = Arc::clone(&self.stop_signal);
            let ctx_clone = ctx.clone();

            thread::spawn(move || {
                loop {
                    // 检查停止信号
                    if let Ok(should_stop) = stop_signal.lock() {
                        if *should_stop {
                            break;
                        }
                    }

                    // 获取间隔时间并休眠
                    let interval_secs = {
                        if let Ok(shared) = shared_state.lock() {
                            if !shared.enable_scheduler {
                                break; // 如果定时任务被关闭，退出循环
                            }
                            shared.interval_hours * 3600
                        } else {
                            3600 // 默认1小时
                        }
                    };
                    
                    // 防止间隔时间过小
                    let interval_secs = interval_secs.max(3600); // 最小1小时

                    // 分段休眠，每秒检查一次停止信号，避免长时间阻塞
                    for _ in 0..interval_secs {
                        thread::sleep(Duration::from_secs(1));
                        if let Ok(should_stop) = stop_signal.lock() {
                            if *should_stop {
                                return;
                            }
                        }
                    }

                    // 执行定时更新
                    {
                        if let Ok(mut shared) = shared_state.lock() {
                            if shared.enable_scheduler {
                                let now = chrono::Local::now();
                                let result = download_and_replace(&shared.url);
                                let record = match result {
                                    Ok(_) => format!("{}: 定时更新成功", now.format("%Y-%m-%d %H:%M:%S")),
                                    Err(e) => format!("{}: 定时更新失败 - {}", now.format("%Y-%m-%d %H:%M:%S"), e),
                                };
                                shared.history.push(record);
                            }
                        }
                    }

                    ctx_clone.request_repaint();
                }
            });
        } else if !self.enable_scheduler {
            // 如果定时任务开关关闭，只执行一次更新
            self.running = false;
            self.status = "手动更新完成".to_string();
            self.next_update_time = None;

            // 如果定时任务正在运行，停止它
            if self.scheduler_running {
                self.stop_scheduler();
            }
        }
    }

    fn stop_scheduler(&mut self) {
        if self.scheduler_running {
            if let Ok(mut stop) = self.stop_signal.lock() {
                *stop = true;
            }
            self.scheduler_running = false;
        }
        self.running = false;
        self.status = "已停止".to_string();
        self.next_update_time = None;
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理窗口显示状态
        self.handle_window_visibility(ctx);
        
        // 处理窗口关闭事件 - 如果启用了后台运行，最小化到后台而不是退出
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.minimize_to_background {
                self.show_window = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                return;
            } else {
                // 如果没有启用后台运行，正常退出
                std::process::exit(0);
            }
        }
        
        // 只在窗口可见时绘制UI
        if !self.show_window {
            std::thread::sleep(Duration::from_millis(100)); // 减少CPU使用率
            ctx.request_repaint_after(Duration::from_millis(500)); // 降低刷新频率
            return;
        }
        
        // 设置应用样式
        let mut style = (*ctx.style()).clone();
        style.visuals.window_rounding = Rounding::same(16.0);
        style.visuals.panel_fill = CARD_COLOR;
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        ctx.set_style(style);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BACKGROUND_COLOR))
            .show(ctx, |ui| {
                // 主容器
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add_space(20.0);

                        // 应用窗口 - 修复边距问题
                        egui::Frame::none()
                            .fill(CARD_COLOR)
                            .rounding(Rounding::same(20.0))
                            .shadow(egui::epaint::Shadow {
                                offset: egui::vec2(0.0, 8.0),
                                blur: 32.0,
                                spread: 0.0,
                                color: Color32::from_rgba_unmultiplied(0, 0, 0, 25),
                            })
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60)))
                            .show(ui, |ui| {
                                let total_width = 720.0;
                                ui.set_width(total_width); // 使用 set_width 而不是 set_max_width 确保精确宽度

                                // 头部 - 确保完全覆盖容器宽度
                                self.draw_header(ui, total_width);

                                ui.add_space(30.0); // 增加头部与内容的间距

                                // 两栏内容区域 - 统一边距
                                ui.allocate_ui_with_layout(
                                    egui::vec2(total_width - 80.0, ui.available_height()), // 左右各40px边距
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        // 左栏 - 操作设置 (40% 宽度)
                                        let content_width = total_width - 80.0;
                                        let left_width = content_width * 0.4;
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(left_width, ui.available_height()),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                self.draw_left_panel(ui, ctx);
                                            }
                                        );

                                        ui.add_space(25.0); // 左右栏间距

                                        // 右栏 - 状态和历史 (60% 宽度减去间距)
                                        let right_width = content_width * 0.6 - 25.0;
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(right_width, ui.available_height()),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                self.draw_right_panel(ui);
                                            }
                                        );
                                    }
                                );

                                ui.add_space(30.0); // 底部边距与顶部保持一致
                            });

                        ui.add_space(20.0);
                    }
                );

                // 同步 shared_state 到本地 history，更新 UI
                if let Ok(shared) = self.shared_state.lock() {
                    self.history = shared.history.clone();
                }
                // 新增代码：
                if self.url != self.previous_url {
                    save_url_to_config(&self.url);
                    self.previous_url = self.url.clone();
                }
            });
    }
}

fn download_and_replace(url: &str) -> Result<(), String> {
    // URL 验证
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid URL: must start with http:// or https://".to_string());
    }
    
    // 添加超时和用户代理
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("AutoUpdateMavenSettings/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP 错误: {}", resp.status()));
    }
    let content = resp.bytes().map_err(|e| e.to_string())?;

    let path = get_m2_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // 备份现有文件
    if path.exists() {
        let backup_path = path.with_extension("xml.backup");
        if let Err(e) = fs::copy(&path, &backup_path) {
            eprintln!("Warning: Failed to create backup: {}", e);
        }
    }
    
    let mut file = fs::File::create(&path).map_err(|e| e.to_string())?;
    file.write_all(&content).map_err(|e| e.to_string())?;
    Ok(())
}

// 通知已存在的实例显示窗口
fn show_existing_window() {
    // 创建一个信号文件来通知已存在的实例
    let signal_path = get_signal_file_path();
    if let Ok(mut file) = std::fs::File::create(&signal_path) {
        let _ = file.write_all(b"show_window");
    }
}

// 获取信号文件路径
fn get_signal_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(".msettings_show_signal")
}

// 检查是否有显示窗口的信号
fn check_show_signal() -> bool {
    let signal_path = get_signal_file_path();
    if signal_path.exists() {
        let _ = std::fs::remove_file(&signal_path); // 移除信号文件
        return true;
    }
    false
}

fn load_window_icon() -> Option<egui::IconData> {
    let icon_paths = [
        "assets/icon/mavi_icon_shadow.png",
        "../Resources/assets/icon/mavi_icon_shadow.png",
        "Contents/Resources/assets/icon/mavi_icon_shadow.png",
    ];
    
    for icon_path in &icon_paths {
        if std::path::Path::new(icon_path).exists() {
            if let Ok(image_bytes) = std::fs::read(icon_path) {
                if let Ok(image) = image::load_from_memory(&image_bytes) {
                    let rgba_image = image.to_rgba8();
                    let (width, height) = rgba_image.dimensions();
                    let pixels = rgba_image.into_raw();
                    
                    return Some(egui::IconData {
                        rgba: pixels,
                        width: width as u32,
                        height: height as u32,
                    });
                }
            }
        }
    }
    None
}

fn main() {
    // 设置panic handler用于更好的调试信息
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("Panic occurred in file '{}' at line {}", location.file(), location.line());
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("Panic payload: {}", s);
        }
    }));
    
    // 打印启动信息用于调试
    eprintln!("Starting MSettings application...");
    
    // 单实例检测 - 添加错误处理
    let instance = match SingleInstance::new("msettings-maven-updater") {
        Ok(instance) => {
            eprintln!("Single instance created successfully");
            Some(instance)
        }
        Err(e) => {
            eprintln!("Failed to create single instance: {}", e);
            // 如果单实例创建失败，继续运行但不做单实例检查
            None
        }
    };
    
    if let Some(ref instance) = instance {
        if !instance.is_single() {
            // 如果已有实例在运行，尝试通知它显示窗口
            show_existing_window();
            std::process::exit(0);
        }
    }
    
    eprintln!("Creating application state...");
    let app = AppState::default();
    eprintln!("Application state created successfully");
    
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([760.0, 750.0])  // 增加高度以适应后台运行开关
        .with_min_inner_size([720.0, 710.0])
        .with_max_inner_size([840.0, 850.0])
        .with_resizable(true);
    
    // 设置窗口图标 - 添加错误处理
    match load_window_icon() {
        Some(icon) => {
            viewport_builder = viewport_builder.with_icon(icon);
        }
        None => {
            eprintln!("Warning: Could not load window icon, using default");
        }
    }
    
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "MSettings - Maven 配置自动更新工具",
        native_options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Box::new(app)
        }),
    ) {
        eprintln!("Failed to run application: {}", e);
        std::process::exit(1);
    }
}