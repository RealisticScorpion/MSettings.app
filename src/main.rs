// main.rs
#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

use auto_launch::AutoLaunch;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, Context, Color32, Stroke, Rounding};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
                    "chinese_font".to_owned(),
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
            .insert(0, "chinese_font".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "chinese_font".to_owned());
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
            .insert(0, fallback_font.to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, fallback_font.to_owned());
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
    interval_hours: u64,
    last_update: Option<Instant>,
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
    last_immediate_update: Option<Instant>,
    next_update_time: Option<chrono::DateTime<chrono::Local>>,
}

struct SharedState {
    enable_scheduler: bool,
    url: String,
    interval_hours: u64,
    history: Vec<String>,
    stop_signal: bool,
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
        let exe_path = std::env::current_exe().unwrap();
        let exe_path_str = exe_path.to_str().unwrap();
        let auto_launch = AutoLaunch::new(app_name, exe_path_str, false, &[] as &[&str]);

        let auto_launch_enabled = auto_launch.is_enabled().unwrap_or(false);

        let shared_state = Arc::new(Mutex::new(SharedState {
            enable_scheduler: false,
            url: "http://13.48.27.126/settings.xml".to_owned(),
            interval_hours: 10,
            history: Vec::new(),
            stop_signal: false,
        }));

        Self {
            url: "http://13.48.27.126/settings.xml".to_owned(),
            interval_hours: 10,
            last_update: None,
            status: "未开始".to_owned(),
            running: false,
            enable_scheduler: false,
            history: Vec::new(),
            shared_state,
            auto_launch_enabled,
            auto_launch,
            scheduler_running: false,
            stop_signal: Arc::new(Mutex::new(false)),
            last_immediate_update: None,
            next_update_time: None,
        }
    }
}

impl AppState {
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
                                // 开发环境路径
                                "assets/icon/mavi_icon_shadow.png",
                                // macOS 应用包内路径
                                "../Resources/assets/icon/mavi_icon_shadow.png",
                                "Contents/Resources/assets/icon/mavi_icon_shadow.png",
                                // 相对于可执行文件的路径
                                &format!("{}/assets/icon/mavi_icon_shadow.png", current_dir.display()),
                                &format!("{}/Contents/Resources/assets/icon/mavi_icon_shadow.png", exe_dir.display()),
                                &format!("{}/../Resources/assets/icon/mavi_icon_shadow.png", exe_dir.display()),
                            ];

                            let mut logo_loaded = false;
                            for logo_path in &logo_paths {
                                if std::path::Path::new(logo_path).exists() {
                                    if let Ok(image_bytes) = std::fs::read(logo_path) {
                                        if let Ok(image) = image::load_from_memory(&image_bytes) {
                                            let rgba_image = image.to_rgba8();
                                            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                                            let pixels = rgba_image.into_raw();
                                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                            let texture_handle = ui.ctx().load_texture("logo", color_image, egui::TextureOptions::default());

                                            ui.add(egui::Image::from_texture(&texture_handle).fit_to_exact_size(egui::vec2(72.0, 72.0)));
                                            ui.add_space(12.0);
                                            logo_loaded = true;
                                            println!("✅ Logo 加载成功: {}", logo_path);
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
                                ui.horizontal(|ui| {
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
                        ui.horizontal(|ui| {
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

        // 同步 URL 到共享状态
        if let Ok(mut shared) = self.shared_state.lock() {
            shared.url = self.url.clone();
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

        ui.horizontal(|ui| {
            // 垂直居中的数值输入框
            ui.allocate_ui_with_layout(
                egui::vec2(75.0, 36.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_sized(
                        [75.0, 36.0],
                        egui::DragValue::new(&mut self.interval_hours)
                            .clamp_range(1..=168)
                    );
                }
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("小时")
                    .size(13.0)
                    .color(SECONDARY_TEXT_COLOR)
            );

            // 同步到共享状态
            if let Ok(mut shared) = self.shared_state.lock() {
                shared.interval_hours = self.interval_hours;
            }
        });

        ui.add_space(20.0);

        // 功能开关
        self.draw_section_title(ui, "功能开关");

        if self.draw_custom_switch(ui, "定时任务", self.enable_scheduler).clicked() {
            self.enable_scheduler = !self.enable_scheduler;
            if let Ok(mut shared) = self.shared_state.lock() {
                shared.enable_scheduler = self.enable_scheduler;
            }
        }

        ui.add_space(10.0);

        if self.draw_custom_switch(ui, "开机自启", self.auto_launch_enabled).clicked() {
            self.auto_launch_enabled = !self.auto_launch_enabled;
            if self.auto_launch_enabled {
                if let Err(_) = self.auto_launch.enable() {
                    self.auto_launch_enabled = false;
                }
            } else {
                let _ = self.auto_launch.disable();
            }
        }

        ui.add_space(20.0);

        // 操作按钮
        self.draw_section_title(ui, "操作控制");

        if !self.running {
            if self.draw_custom_button(ui, "启动自动更新", true, true).clicked() {
                if self.url.starts_with("http") && self.interval_hours > 0 {
                    self.start_update_task(ctx);
                } else {
                    self.status = "请输入有效的 URL 和间隔".to_owned();
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
        // 立即执行一次更新
        self.perform_immediate_update();

        if self.enable_scheduler && !self.scheduler_running {
            // 设置下次更新时间
            let now = chrono::Local::now();
            self.next_update_time = Some(now + chrono::Duration::hours(self.interval_hours as i64));

            // 启动定时任务
            self.scheduler_running = true;
            self.running = true;
            self.status = "定时任务已启动".to_owned();

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

                    // 分段休眠，每秒检查一次停止信号
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
            self.status = "手动更新完成".to_owned();
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
        self.status = "已停止".to_owned();
        self.next_update_time = None;
    }

    fn perform_immediate_update(&mut self) {
        if !self.url.starts_with("http") {
            self.status = "请输入有效的下载地址".to_owned();
            return;
        }

        let now = chrono::Local::now();
        let result = download_and_replace(&self.url);
        let record = match result {
            Ok(_) => {
                self.last_immediate_update = Some(Instant::now());
                self.status = if self.enable_scheduler {
                    "立即更新成功，定时任务已启动".to_owned()
                } else {
                    "手动更新成功".to_owned()
                };
                format!("{}: 立即更新成功", now.format("%Y-%m-%d %H:%M:%S"))
            },
            Err(e) => {
                self.status = format!("更新失败: {}", e);
                format!("{}: 立即更新失败 - {}", now.format("%Y-%m-%d %H:%M:%S"), e)
            },
        };

        self.history.push(record.clone());

        // 同步到共享状态
        if let Ok(mut shared) = self.shared_state.lock() {
            shared.history.push(record);
            shared.url = self.url.clone();
            shared.interval_hours = self.interval_hours;
            shared.enable_scheduler = self.enable_scheduler;
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                // 同步 shared_state 到本地 history
                if let Ok(shared) = self.shared_state.lock() {
                    self.history = shared.history.clone();
                }
            });
    }
}

fn download_and_replace(url: &str) -> Result<(), String> {
    let resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP 错误: {}", resp.status()));
    }
    let content = resp.bytes().map_err(|e| e.to_string())?;

    let path = get_m2_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::File::create(&path).map_err(|e| e.to_string())?;
    file.write_all(&content).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let app = AppState::default();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 680.0])  // 增加窗口尺寸以适应更大的背景
            .with_min_inner_size([720.0, 640.0])
            .with_max_inner_size([840.0, 780.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "MSettings - Maven 配置自动更新工具",
        native_options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Box::new(app)
        }),
    ).unwrap();
}