#!/usr/bin/env python3
"""
创建最简单的ICO文件，用于Windows资源编译
"""

import struct
import os

def create_minimal_ico():
    """创建一个最小的、符合Windows 3.00格式的ICO文件"""
    output_file = "assets/icon/app_icon.ico"
    
    # 创建一个16x16的简单图标
    width = height = 16
    
    # ICO文件头 (6 bytes)
    ico_header = struct.pack('<HHH', 
        0,    # Reserved, must be 0
        1,    # Type: 1 for ICO
        1     # Number of images
    )
    
    # 创建16x16的像素数据 (BGRA格式)
    pixels = bytearray()
    for y in range(height):
        for x in range(width):
            # 创建一个简单的蓝色圆形图标
            dx, dy = x - 8, y - 8
            if dx*dx + dy*dy <= 36:  # 圆形
                pixels.extend([255, 100, 100, 255])  # BGRA: 蓝色，不透明
            else:
                pixels.extend([0, 0, 0, 0])  # BGRA: 透明
    
    # BMP信息头 (40 bytes)
    bmp_header = struct.pack('<IIIHHIIIIII',
        40,                    # Header size
        width,                 # Width
        height * 2,            # Height (doubled for ICO)
        1,                     # Planes
        32,                    # Bits per pixel
        0,                     # Compression
        len(pixels),           # Image size
        0, 0, 0, 0            # X/Y pixels per meter, colors used, important colors
    )
    
    # AND遮罩 (1 bit per pixel, padded to 4-byte boundary)
    and_mask_size = ((width + 31) // 32) * 4 * height
    and_mask = bytearray(and_mask_size)
    
    # 计算图像数据总大小
    image_data_size = len(bmp_header) + len(pixels) + len(and_mask)
    
    # 图像目录条目 (16 bytes)
    directory_entry = struct.pack('<BBBBHHII',
        width if width < 256 else 0,   # Width (0 means 256)
        height if height < 256 else 0, # Height (0 means 256)
        0,                             # Color count (0 for >8bpp)
        0,                             # Reserved
        1,                             # Color planes
        32,                            # Bits per pixel
        image_data_size,               # Size of image data
        6 + 16                         # Offset to image data
    )
    
    # 确保目录存在
    os.makedirs(os.path.dirname(output_file), exist_ok=True)
    
    # 写入ICO文件
    with open(output_file, 'wb') as f:
        f.write(ico_header)
        f.write(directory_entry)
        f.write(bmp_header)
        f.write(pixels)
        f.write(and_mask)
    
    print(f"✅ 创建最小ICO文件: {output_file}")
    print(f"   尺寸: {width}x{height}, 格式: 32位BGRA")
    return True

if __name__ == "__main__":
    create_minimal_ico()