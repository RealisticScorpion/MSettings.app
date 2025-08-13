@echo off
echo 🚀 开始构建 MSettings for Windows...

REM 清理旧构建
cargo clean

REM 检查 Rust
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ 未安装 Rust 工具链
    exit /b 1
)

REM 构建
cargo build --release
if %errorlevel% neq 0 (
    echo ❌ 构建失败!
    exit /b 1
)

REM 创建 dist 目录结构
mkdir dist\assets\fonts 2>nul
mkdir dist\assets\icon 2>nul

REM 复制主程序
copy target\release\AutoUpdateMavenSettings.exe dist\

REM 拷贝字体
IF EXIST assets\fonts\SourceHanSerifCN-Regular-1.otf (
    copy assets\fonts\SourceHanSerifCN-Regular-1.otf dist\assets\fonts\
) ELSE (
    echo ⚠️  未找到中文字体文件
)

REM 拷贝图标
IF EXIST assets\icon\mavi_icon_shadow.png (
    copy assets\icon\mavi_icon_shadow.png dist\assets\icon\
    echo ✅ 图标文件已复制
) ELSE (
    echo ⚠️  未找到图标文件 mavi_icon_shadow.png
)

echo ✅ 打包完成，输出目录：dist\
pause