#!/usr/bin/env python3
"""
将JPG图片缩小并转换为RGB565格式的TGA
"""

import sys
import struct
from pathlib import Path
from typing import Optional, Tuple
from PIL import Image


def convert_jpg_to_tga_rgb565(input_path: str, output_path: Optional[str] = None, size: Tuple[int, int] = (64, 64)) -> str:
    """
    将JPG图片缩小并转换为RGB565格式的TGA
    
    Args:
        input_path: 输入的JPG图片路径
        output_path: 输出的TGA图片路径（可选，默认为输入文件名.tga）
        size: 目标尺寸，默认为(64, 64)
    
    Returns:
        输出文件的路径
    """
    input_path_obj = Path(input_path)
    
    if not input_path_obj.exists():
        raise FileNotFoundError(f"找不到输入文件: {input_path}")
    
    # 确定输出路径
    if output_path is None:
        output_path_obj = input_path_obj.with_suffix('.tga')
    else:
        output_path_obj = Path(output_path)
    
    output_path_str = str(output_path_obj)
    
    # 打开图片
    with Image.open(input_path_obj) as img:
        # 转换为RGB模式
        if img.mode != 'RGB':
            img = img.convert('RGB')
        
        # 缩小到指定尺寸
        img_resized = img.resize(size, Image.Resampling.LANCZOS)
        
        # 转换为RGB565格式（每个像素16位）
        # 用于tinytga的Rgb565类型
        # RGB565布局: bit15-11=红色(5位), bit10-5=绿色(6位), bit4-0=蓝色(5位)
        pixel_data = bytearray()
        for r, g, b in img_resized.getdata():
            r5 = (r >> 3) & 0x1F   # 红色: 8位转5位
            g6 = (g >> 2) & 0x3F   # 绿色: 8位转6位
            b5 = (b >> 3) & 0x1F   # 蓝色: 8位转5位
            # 组合成16位RGB565: 高位->低位 = RRRRR GGGGGG BBBBB
            pixel565 = (r5 << 11) | (g6 << 5) | b5
            pixel_data.append(pixel565 & 0xFF)
            pixel_data.append((pixel565 >> 8) & 0xFF)
    
    header = bytearray(18)
    header[0] = 0    # ID length
    header[1] = 0    # Color map type
    header[2] = 2    # Image type: uncompressed true-color
    header[3] = 0    # Color map spec: first entry low
    header[4] = 0    # Color map spec: first entry high
    header[5] = 0    # Color map spec: length low
    header[6] = 0    # Color map spec: length high
    header[7] = 0    # Color map spec: depth
    header[8] = 0    # X origin low
    header[9] = 0    # X origin high
    header[10] = 0   # Y origin low
    header[11] = 0   # Y origin high
    header[12] = size[0] & 0xFF      # Width low
    header[13] = (size[0] >> 8) & 0xFF  # Width high
    header[14] = size[1] & 0xFF      # Height low
    header[15] = (size[1] >> 8) & 0xFF  # Height high
    header[16] = 16  # Bits per pixel
    header[17] = 0x20  # Image descriptor: bit 5=1 (origin at bottom-left)
    
    # 写入TGA文件
    with open(output_path_str, 'wb') as f:
        f.write(header)
        f.write(pixel_data)
    
    return output_path_str


def main():
    if len(sys.argv) < 2:
        print("用法: python convert_to_tga.py <输入jpg文件> [输出tga文件]")
        print("示例: python convert_to_tga.py jing.jpg")
        print("      python convert_to_tga.py jing.jpg output.tga")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None
    
    try:
        result = convert_jpg_to_tga_rgb565(input_file, output_file, (128, 128))
        print(f"转换成功! 输出文件: {result}")
        print("格式: RGB565 (16位色深, tinytga的Rgb565类型)")
        print()
        print("提示: main.rs中请使用 Tga<Rgb565> 类型")
    except FileNotFoundError as e:
        print(f"错误: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"转换失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
