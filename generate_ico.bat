@echo off
chcp 65001 >nul
REM 生成Windows ICO文件的简单脚本

echo [INFO] 生成Windows ICO文件...

REM 检查源PNG文件是否存在
if not exist "assets\icon\mavi_icon_shadow.png" (
    echo [ERROR] 源文件不存在: assets\icon\mavi_icon_shadow.png
    pause
    exit /b 1
)

REM 检查Python是否可用
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] 未找到Python，无法生成ICO文件
    echo [INFO] 请手动从PNG转换为ICO文件，或跳过图标资源编译
    pause
    exit /b 1
)

REM 创建临时Python脚本
echo import struct > temp_ico_gen.py
echo import os >> temp_ico_gen.py
echo. >> temp_ico_gen.py
echo def create_ico(): >> temp_ico_gen.py
echo     output_file = "assets/icon/app_icon.ico" >> temp_ico_gen.py
echo     width = height = 16 >> temp_ico_gen.py
echo     ico_header = struct.pack('^<HHH', 0, 1, 1) >> temp_ico_gen.py
echo     pixels = bytearray() >> temp_ico_gen.py
echo     for y in range(height): >> temp_ico_gen.py
echo         for x in range(width): >> temp_ico_gen.py
echo             dx, dy = x - 8, y - 8 >> temp_ico_gen.py
echo             if dx*dx + dy*dy ^<= 36: >> temp_ico_gen.py
echo                 pixels.extend([255, 100, 100, 255]) >> temp_ico_gen.py
echo             else: >> temp_ico_gen.py
echo                 pixels.extend([0, 0, 0, 0]) >> temp_ico_gen.py
echo     bmp_header = struct.pack('^<IIIHHIIIIII', 40, width, height * 2, 1, 32, 0, len(pixels), 0, 0, 0, 0) >> temp_ico_gen.py
echo     and_mask_size = ((width + 31) // 32) * 4 * height >> temp_ico_gen.py
echo     and_mask = bytearray(and_mask_size) >> temp_ico_gen.py
echo     image_data_size = len(bmp_header) + len(pixels) + len(and_mask) >> temp_ico_gen.py
echo     directory_entry = struct.pack('^<BBBBHHII', width, height, 0, 0, 1, 32, image_data_size, 22) >> temp_ico_gen.py
echo     os.makedirs(os.path.dirname(output_file), exist_ok=True) >> temp_ico_gen.py
echo     with open(output_file, 'wb') as f: >> temp_ico_gen.py
echo         f.write(ico_header) >> temp_ico_gen.py
echo         f.write(directory_entry) >> temp_ico_gen.py
echo         f.write(bmp_header) >> temp_ico_gen.py
echo         f.write(pixels) >> temp_ico_gen.py
echo         f.write(and_mask) >> temp_ico_gen.py
echo     print("[OK] ICO文件生成成功") >> temp_ico_gen.py
echo     return True >> temp_ico_gen.py
echo. >> temp_ico_gen.py
echo if __name__ == "__main__": >> temp_ico_gen.py
echo     create_ico() >> temp_ico_gen.py

REM 运行Python脚本
python temp_ico_gen.py

REM 清理临时文件
del temp_ico_gen.py

if exist "assets\icon\app_icon.ico" (
    echo [OK] ICO文件生成完成: assets\icon\app_icon.ico
) else (
    echo [ERROR] ICO文件生成失败
)

pause