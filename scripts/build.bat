# Windows 构建脚本 (build.bat)
# @echo off
# echo 🚀 开始构建 MSettings...
#
# where cargo >nul 2>nul
# if %errorlevel% neq 0 (
#     echo ❌ 错误: 未找到 Cargo，请先安装 Rust 工具链
#     pause
#     exit /b 1
# )
#
# if not exist "assets\fonts\SourceHanSerifCN-Regular-1.otf" (
#     echo ⚠️  警告: 未找到中文字体文件，程序可能显示乱码
#     echo    请将字体文件放置在: assets\fonts\SourceHanSerifCN-Regular-1.otf
# )
#
# if not exist "assets\icon\mavi_icon_shadow.png" (
#     echo ⚠️  警告: 未找到应用图标文件
#     echo    请将图标文件放置在: assets\icon\mavi_icon_shadow.png
# )
#
# mkdir assets\fonts 2>nul
# mkdir assets\icon 2>nul
#
# echo 🧹 清理之前的构建...
# cargo clean
#
# echo 🔨 构建 Release 版本...
# cargo build --release
#
# if %errorlevel% equ 0 (
#     echo ✅ 构建成功!
#     echo 📦 可执行文件位置: target\release\AutoUpdateMavenSettings.exe
#
#     echo 📋 创建分发包...
#     rmdir /s /q dist 2>nul
#     mkdir dist
#     mkdir dist\MSettings
#
#     copy target\release\AutoUpdateMavenSettings.exe dist\MSettings\
#     if exist assets xcopy /e /i assets dist\MSettings\assets\
#
#     REM 创建快捷方式和图标设置
#     if exist "assets\icon\mavi_icon_shadow.png" (
#         copy "assets\icon\mavi_icon_shadow.png" "dist\MSettings\MSettings.png"
#     )
#
#     echo 📁 Windows 分发包已创建在 dist\MSettings\ 目录中
# ) else (
#     echo ❌ 构建失败!
#     pause
#     exit /b 1
# )