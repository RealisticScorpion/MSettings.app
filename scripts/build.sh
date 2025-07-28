#!/bin/bash

# build.sh - macOS/Linux 构建脚本

echo "🚀 开始构建 MSettings..."

# 检查 Rust 环境
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误: 未找到 Cargo，请先安装 Rust 工具链"
    exit 1
fi

# 检查字体文件
if [ ! -f "assets/fonts/SourceHanSerifCN-Regular-1.otf" ]; then
    echo "⚠️  警告: 未找到中文字体文件，程序可能显示乱码"
    echo "   请将字体文件放置在: assets/fonts/SourceHanSerifCN-Regular-1.otf"
fi

# 检查图标文件
if [ ! -f "assets/icon/mavi_icon_shadow.png" ]; then
    echo "⚠️  警告: 未找到应用图标文件"
    echo "   请将图标文件放置在: assets/icon/mavi_icon_shadow.png"
fi

# 创建 assets 目录
mkdir -p assets/fonts
mkdir -p assets/icon

# 清理之前的构建
echo "🧹 清理之前的构建..."
cargo clean

# 构建 release 版本
echo "🔨 构建 Release 版本..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ 构建成功!"
    echo "📦 可执行文件位置: target/release/AutoUpdateMavenSettings"

    # 创建分发目录
    echo "📋 创建分发包..."
    rm -rf dist
    mkdir -p dist

    # macOS 应用包
    if [[ "$OSTYPE" == "darwin"* ]]; then
        mkdir -p dist/MSettings.app/Contents/MacOS
        mkdir -p dist/MSettings.app/Contents/Resources

        # 复制可执行文件
        cp target/release/AutoUpdateMavenSettings dist/MSettings.app/Contents/MacOS/

        # 复制资源文件
        if [ -d "assets" ]; then
            cp -r assets dist/MSettings.app/Contents/Resources/
        fi

        # 复制应用图标
        if [ -f "assets/icon/mavi_icon_shadow.png" ]; then
            # 转换为 icns 格式 (需要安装 iconutil)
            if command -v iconutil &> /dev/null; then
                mkdir -p dist/MSettings.app/Contents/Resources/AppIcon.iconset
                cp assets/icon/mavi_icon_shadow.png dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_512x512.png
                iconutil -c icns dist/MSettings.app/Contents/Resources/AppIcon.iconset
                rm -rf dist/MSettings.app/Contents/Resources/AppIcon.iconset
            fi
        fi

        # 创建 Info.plist
        cat > dist/MSettings.app/Contents/Info.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>AutoUpdateMavenSettings</string>
    <key>CFBundleIdentifier</key>
    <string>com.msettings.app</string>
    <key>CFBundleName</key>
    <string>MSettings</string>
    <key>CFBundleDisplayName</key>
    <string>MSettings</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
</dict>
</plist>
EOF

        echo "📁 macOS 应用包已创建：dist/MSettings.app"
    else
        # Linux 分发包
        mkdir -p dist/MSettings
        cp target/release/AutoUpdateMavenSettings dist/MSettings/
        if [ -d "assets" ]; then
            cp -r assets dist/MSettings/
        fi
        echo "📁 Linux 分发包已创建：dist/MSettings/"
    fi

else
    echo "❌ 构建失败!"
    exit 1
fi