@echo off
chcp 65001 >nul
echo 开始构建 MSettings for Windows...

REM 检查 Rust
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] 未安装 Rust 工具链
    exit /b 1
)

REM 检查图标文件
echo [INFO] 检查图标文件...
if exist "assets\icon\app_icon.ico" (
    echo [OK] ICO文件已存在
) else (
    echo [WARN] ICO文件不存在，将使用默认图标
)

REM 清理旧构建
echo [INFO] 清理之前的构建...
cargo clean

REM 构建
echo [INFO] 构建 Release 版本...
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] 构建失败!
    exit /b 1
)

REM 创建分发包
echo [INFO] 创建分发包...
if exist dist rmdir /s /q dist
mkdir dist
mkdir dist\assets 2>nul
mkdir dist\assets\fonts 2>nul
mkdir dist\assets\icon 2>nul

REM 复制主程序
echo [INFO] 复制可执行文件...
if exist "target\release\AutoUpdateMavenSettings.exe" (
    copy "target\release\AutoUpdateMavenSettings.exe" dist\
    echo [OK] 可执行文件已复制
) else (
    echo [ERROR] 未找到可执行文件！请先运行 cargo build --release
    pause
    exit /b 1
)

REM 拷贝字体
echo [INFO] 复制字体文件...
IF EXIST "assets\fonts\SourceHanSerifCN-Regular-1.otf" (
    copy "assets\fonts\SourceHanSerifCN-Regular-1.otf" "dist\assets\fonts\"
    echo [OK] 字体文件已复制
) ELSE (
    echo [WARN] 未找到中文字体文件
)

REM 拷贝图标
echo [INFO] 复制图标文件...
IF EXIST "assets\icon\mavi_icon_shadow.png" (
    copy "assets\icon\mavi_icon_shadow.png" "dist\assets\icon\"
    echo [OK] 图标文件已复制
) ELSE (
    echo [WARN] 未找到图标文件 mavi_icon_shadow.png
)

echo.
echo [SUCCESS] 构建完成！
echo 分发包位置：dist\
echo 包含文件：
dir /b dist
echo.
pause