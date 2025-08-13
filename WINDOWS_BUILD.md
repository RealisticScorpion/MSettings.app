# Windows 构建说明

## 问题解决

如果遇到图标资源编译错误：`error RC2175: resource file assets\icon\app_icon.ico is not in 3.00 format`

### 解决方案

1. **自动解决（推荐）**：
   ```cmd
   scripts\build.bat
   ```
   构建脚本会自动检查并生成正确格式的ICO文件。

2. **手动生成ICO文件**：
   ```cmd
   python create_simple_ico.py
   ```

3. **备用方案**：
   如果仍有问题，可以：
   - 删除 `app_icon.rc` 文件（应用会使用默认图标）
   - 或使用备用RC文件：`copy app_icon_fallback.rc app_icon.rc`

## 构建步骤

### 完整构建（推荐）
```cmd
scripts\build.bat
```

### 手动构建
```cmd
# 1. 生成图标（如需要）
python create_simple_ico.py

# 2. 构建应用
cargo build --release

# 3. 创建分发包
mkdir dist\assets\fonts
mkdir dist\assets\icon
copy target\release\AutoUpdateMavenSettings.exe dist\
copy assets\icon\mavi_icon_shadow.png dist\assets\icon\
copy assets\fonts\SourceHanSerifCN-Regular-1.otf dist\assets\fonts\
```

## 故障排除

### ICO文件格式错误
- 确保使用 `create_simple_ico.py` 生成的ICO文件
- 检查文件是否存在：`dir assets\icon\app_icon.ico`

### 缺少Python
如果没有Python，可以：
1. 从现有项目复制ICO文件
2. 使用在线ICO转换工具
3. 跳过图标编译（删除app_icon.rc）

### 权限问题
以管理员身份运行命令提示符：
```cmd
# 右键点击"命令提示符" -> "以管理员身份运行"
```

## 分发

构建完成后，分发包位于 `dist\` 目录：
```
dist\
├── AutoUpdateMavenSettings.exe
└── assets\
    ├── fonts\
    └── icon\
```

可以将整个 `dist` 文件夹打包分发给用户。