#!/usr/bin/env python3
"""
将JPG图片转换为RGB565格式的BMP
使用BITFIELDS格式，与tinybmp示例兼容
"""

import sys
import struct
from pathlib import Path
from typing import Optional, Tuple
from PIL import Image


def convert_jpg_to_bmp_rgb565(input_path: str, output_path: Optional[str] = None, size: Tuple[int, int] = (128,128)) -> str:
    """
    将JPG图片转换为16位RGB565格式的BMP（BITFIELDS格式）
    
    Args:
        input_path: 输入的JPG图片路径
        output_path: 输出的BMP图片路径（可选，默认为输入文件名.bmp）
        size: 目标尺寸，默认为64x64
    
    Returns:
        输出文件的路径
    """
    input_path_obj = Path(input_path)
    
    if not input_path_obj.exists():
        raise FileNotFoundError(f"找不到输入文件: {input_path}")
    
    # 确定输出路径
    if output_path is None:
        output_path_obj = input_path_obj.with_suffix('.bmp')
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
        
        # 获取所有像素数据
        all_pixels = list(img_resized.getdata())
        width, height = size
        
        # BMP是bottom-up格式，需要从底部开始存储行
        # 即第一行像素数据对应图像的最底部
        pixel_data = bytearray()
        for y in range(height - 1, -1, -1):  # 从底部开始
            for x in range(width):
                r, g, b = all_pixels[y * width + x]
                # RGB565: R:5位, G:6位, B:5位
                r5 = (r >> 3) & 0x1F
                g6 = (g >> 2) & 0x3F
                b5 = (b >> 3) & 0x1F
                # 组合成16位: RRRRR GGGGGG BBBBB
                pixel565 = (r5 << 11) | (g6 << 5) | b5
                # 小端序存储
                pixel_data.append(pixel565 & 0xFF)
                pixel_data.append((pixel565 >> 8) & 0xFF)
        
        # 创建BITFIELDS格式的BMP头（70字节）
        header = create_bitfields_bmp_header(size[0], size[1], len(pixel_data))
        
        # 写入BMP文件
        with open(output_path_str, 'wb') as f:
            f.write(header)
            f.write(pixel_data)
    
    return output_path_str


def create_bitfields_bmp_header(width: int, height: int, pixel_data_size: int) -> bytes:
    """
    创建BITFIELDS格式的BMP文件头
    
    格式与tinybmp库的logo-rgb565.bmp兼容:
    - DIB头大小: 56字节
    - 压缩: BI_BITFIELDS (3)
    - RGB565掩码: R=0xF800, G=0x07E0, B=0x001F
    """
    # 文件总大小
    file_size = 70 + pixel_data_size
    
    header = bytearray(70)
    
    # BMP文件头（14字节）
    header[0:2] = b'BM'           # 签名
    header[2:6] = struct.pack('<I', file_size)  # 文件大小
    header[6:10] = struct.pack('<I', 0)  # 保留
    header[10:14] = struct.pack('<I', 70)  # 像素数据偏移 (54 + 16)
    
    # DIB位图信息头（56字节）
    header[14:18] = struct.pack('<I', 56)  # 头大小 (BITMAPV4HEADER)
    header[18:22] = struct.pack('<i', width)  # 宽度
    header[22:26] = struct.pack('<i', height)  # 高度（正值=bottom-up）
    header[26:28] = struct.pack('<H', 1)  # 平面数
    header[28:30] = struct.pack('<H', 16)  # 位深度
    header[30:34] = struct.pack('<I', 3)   # 压缩: BI_BITFIELDS
    header[34:38] = struct.pack('<I', pixel_data_size)  # 像素数据大小
    header[38:42] = struct.pack('<I', 0)  # X像素每米
    header[42:46] = struct.pack('<I', 0)  # Y像素每米
    header[46:50] = struct.pack('<I', 0)  # 颜色表颜色数
    header[50:54] = struct.pack('<I', 0)  # 重要颜色数
    
    # BITFIELDS颜色掩码（16字节，位置54-69）
    header[54:58] = struct.pack('<I', 0x00F800)  # 红色掩码: 1111100000000000 (R5 G6 B5中的R)
    header[58:62] = struct.pack('<I', 0x0007E0)  # 绿色掩码: 00000000011111100000 (R5 G6 B5中的G)
    header[62:66] = struct.pack('<I', 0x00001F)  # 蓝色掩码: 00000000000000011111 (R5 G6 B5中的B)
    
    # 保留（4字节）
    header[66:70] = struct.pack('<I', 0)  # 保留
    
    return bytes(header)


def main():
    if len(sys.argv) < 2:
        print("用法: python convert_to_bmp.py <输入jpg文件> [输出bmp文件]")
        print("示例: python convert_to_bmp.py jing.jpg")
        print("      python convert_to_bmp.py jing.jpg output.bmp")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None
    
    try:
        result = convert_jpg_to_bmp_rgb565(input_file, output_file, (128, 128))
        print(f"转换成功! 输出文件: {result}")
        print("格式: 16位RGB565 (BITFIELDS格式，与tinybmp示例兼容)")
        print()
        print("main.rs中使用:")
        print("  let data = include_bytes!(\"../../output.bmp\");")
        print("  let bmp: ImageRawBmp<Rgb565> = ImageRawBmp::from_slice(data).unwrap();")
    except FileNotFoundError as e:
        print(f"错误: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"转换失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
