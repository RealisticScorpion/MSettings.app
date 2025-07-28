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
    echo "   或者程序将尝试使用系统默认字体"
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
        echo "🍎 创建 macOS 应用包..."
        mkdir -p dist/MSettings.app/Contents/MacOS
        mkdir -p dist/MSettings.app/Contents/Resources
        mkdir -p dist/MSettings.app/Contents/Resources/assets/fonts
        mkdir -p dist/MSettings.app/Contents/Resources/assets/icon

        # 复制可执行文件
        cp target/release/AutoUpdateMavenSettings dist/MSettings.app/Contents/MacOS/
        echo "✅ 可执行文件已复制"

        # 复制字体文件
        if [ -f "assets/fonts/SourceHanSerifCN-Regular-1.otf" ]; then
            cp assets/fonts/SourceHanSerifCN-Regular-1.otf dist/MSettings.app/Contents/Resources/assets/fonts/
            echo "✅ 字体文件已复制"
        else
            echo "⚠️  字体文件未找到，跳过复制"
        fi

        # 复制图标文件
        if [ -f "assets/icon/mavi_icon_shadow.png" ]; then
            cp assets/icon/mavi_icon_shadow.png dist/MSettings.app/Contents/Resources/assets/icon/
            echo "✅ 图标文件已复制"
        else
            echo "⚠️  图标文件未找到，跳过复制"
        fi

        # 转换图标为 icns 格式
        if [ -f "assets/icon/mavi_icon_shadow.png" ]; then
            echo "🎨 转换应用图标..."
            mkdir -p dist/MSettings.app/Contents/Resources/AppIcon.iconset

            # 创建不同尺寸的图标（使用 sips 命令）
            if command -v sips &> /dev/null; then
                echo "📐 生成各种尺寸的图标..."
                sips -z 16 16 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_16x16.png > /dev/null 2>&1
                sips -z 32 32 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_16x16@2x.png > /dev/null 2>&1
                sips -z 32 32 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_32x32.png > /dev/null 2>&1
                sips -z 64 64 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_32x32@2x.png > /dev/null 2>&1
                sips -z 128 128 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_128x128.png > /dev/null 2>&1
                sips -z 256 256 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_128x128@2x.png > /dev/null 2>&1
                sips -z 256 256 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_256x256.png > /dev/null 2>&1
                sips -z 512 512 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_256x256@2x.png > /dev/null 2>&1
                sips -z 512 512 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_512x512.png > /dev/null 2>&1
                sips -z 1024 1024 assets/icon/mavi_icon_shadow.png --out dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_512x512@2x.png > /dev/null 2>&1

                # 使用 iconutil 生成 icns 文件
                if command -v iconutil &> /dev/null; then
                    iconutil -c icns dist/MSettings.app/Contents/Resources/AppIcon.iconset > /dev/null 2>&1
                    if [ $? -eq 0 ]; then
                        echo "✅ 应用图标已转换为 icns 格式"
                        # 清理临时文件
                        rm -rf dist/MSettings.app/Contents/Resources/AppIcon.iconset
                    else
                        echo "❌ iconutil 转换失败"
                    fi
                else
                    echo "⚠️  未找到 iconutil 命令"
                    # 手动复制一个 512x512 作为备选
                    cp dist/MSettings.app/Contents/Resources/AppIcon.iconset/icon_512x512.png dist/MSettings.app/Contents/Resources/AppIcon.icns
                    rm -rf dist/MSettings.app/Contents/Resources/AppIcon.iconset
                fi
            else
                echo "⚠️  未找到 sips 命令，使用原图标"
                cp assets/icon/mavi_icon_shadow.png dist/MSettings.app/Contents/Resources/AppIcon.icns
            fi
        else
            echo "⚠️  图标文件不存在，创建默认图标"
            # 创建一个简单的默认图标（可选）
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
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSAppTransportSecurity</key>
    <dict>
        <key>NSAllowsArbitraryLoads</key>
        <true/>
    </dict>
    <key>LSUIElement</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 MSettings. All rights reserved.</string>
</dict>
</plist>
EOF
        echo "✅ Info.plist 已创建"

        # 设置可执行权限
        chmod +x dist/MSettings.app/Contents/MacOS/AutoUpdateMavenSettings

        # 验证应用包结构
        echo "🔍 验证应用包结构..."
        if [ -f "dist/MSettings.app/Contents/Info.plist" ]; then
            echo "✅ Info.plist 存在"
        fi
        if [ -f "dist/MSettings.app/Contents/MacOS/AutoUpdateMavenSettings" ]; then
            echo "✅ 可执行文件存在"
        fi
        if [ -f "dist/MSettings.app/Contents/Resources/AppIcon.icns" ]; then
            echo "✅ 应用图标存在"
        else
            echo "⚠️  应用图标文件不存在"
        fi
        if [ -f "dist/MSettings.app/Contents/Resources/assets/icon/mavi_icon_shadow.png" ]; then
            echo "✅ UI 图标文件存在"
        else
            echo "⚠️  UI 图标文件不存在"
        fi

        echo "📁 macOS 应用包已创建：dist/MSettings.app"
        echo ""
        echo "🚀 安装说明："
        echo "1. 复制应用到 Applications 目录："
        echo "   cp -r dist/MSettings.app /Applications/"
        echo ""
        echo "2. 如果遇到安全问题，在系统偏好设置中允许运行："
        echo "   系统偏好设置 → 安全性与隐私 → 通用 → 仍要打开"
        echo ""
        echo "3. 或者移除隔离标志："
        echo "   sudo xattr -r -d com.apple.quarantine /Applications/MSettings.app"
        echo ""

        # 自动移除隔离标志（如果在当前目录）
        if [ -w "dist/MSettings.app" ]; then
            echo "🔓 移除隔离标志..."
            xattr -r -d com.apple.quarantine dist/MSettings.app 2>/dev/null || true
            echo "✅ 隔离标志已移除"
        fi

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