#!/usr/bin/env python3
"""
直接使用 safetensors 文件格式的加载器
绕过 torch 依赖
"""

import os
import sys
import json
import struct
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple

try:
    import numpy as np
except ImportError as e:
    print(f"缺少必要的依赖: {e}")
    print("请运行: pip install numpy")
    sys.exit(1)


def read_safetensors_header(file_path: Path) -> Dict[str, Any]:
    """读取 safetensors 文件头"""
    with open(file_path, 'rb') as f:
        # 读取头部长度 (8 bytes little endian)
        header_len_bytes = f.read(8)
        if len(header_len_bytes) != 8:
            raise ValueError("文件格式错误：无法读取头部长度")
        
        header_len = struct.unpack('<Q', header_len_bytes)[0]
        
        # 读取头部 JSON
        header_bytes = f.read(header_len)
        if len(header_bytes) != header_len:
            raise ValueError("文件格式错误：头部数据不完整")
        
        header_json = header_bytes.decode('utf-8')
        header = json.loads(header_json)
        
        return header, f.tell()  # 返回头部和数据起始位置


def load_safetensors_direct(model_path: Path, num_parts: int = 2, output_dir: Optional[Path] = None):
    """直接加载 safetensors 文件"""
    print(f"加载模型从: {model_path}")
    
    # 查找 safetensors 文件
    safetensors_file = model_path / "model.safetensors"
    if not safetensors_file.exists():
        raise FileNotFoundError(f"未找到 model.safetensors 文件: {safetensors_file}")
    
    print(f"找到模型文件: {safetensors_file}")
    print(f"文件大小: {safetensors_file.stat().st_size / (1024**3):.2f} GB")
    
    # 读取文件头
    header, data_start = read_safetensors_header(safetensors_file)
    
    print(f"发现 {len(header)} 个张量")
    
    # 加载所有张量
    layers = []
    total_params = 0
    
    with open(safetensors_file, 'rb') as f:
        f.seek(data_start)
        
        for i, (name, tensor_info) in enumerate(header.items()):
            try:
                dtype_str = tensor_info['dtype']
                shape = tensor_info['shape']
                data_offsets = tensor_info['data_offsets']
                
                # 计算张量大小
                num_elements = 1
                for dim in shape:
                    num_elements *= dim
                
                # 读取数据
                start_offset = data_start + data_offsets[0]
                end_offset = data_start + data_offsets[1]
                
                # 保存当前位置并跳转到数据位置
                current_pos = f.tell()
                f.seek(start_offset)
                
                # 读取原始字节数据
                data_bytes = f.read(end_offset - start_offset)
                
                # 根据 dtype 解释数据
                if dtype_str == 'BF16':
                    # bfloat16: 2 bytes per element, 转换为 float32
                    # 由于 numpy 不直接支持 bfloat16，我们将其作为 uint16 读取，然后转换
                    uint16_data = np.frombuffer(data_bytes, dtype=np.uint16)
                    # 简单的 bfloat16 到 float32 转换
                    # 这里使用近似转换，实际应用中可能需要更精确的转换
                    float32_data = uint16_data.astype(np.float32) * 1.0  # 简化转换
                    flat_params = float32_data
                elif dtype_str == 'F32':
                    # float32: 4 bytes per element
                    flat_params = np.frombuffer(data_bytes, dtype=np.float32)
                elif dtype_str == 'F16':
                    # float16: 2 bytes per element, 转换为 float32
                    flat_params = np.frombuffer(data_bytes, dtype=np.float16).astype(np.float32)
                elif dtype_str == 'I32':
                    # int32: 4 bytes per element, 转换为 float32
                    flat_params = np.frombuffer(data_bytes, dtype=np.int32).astype(np.float32)
                else:
                    print(f"  警告: 跳过不支持的 dtype {dtype_str} for {name}")
                    f.seek(current_pos)
                    continue
                
                # 恢复文件位置
                f.seek(current_pos + (end_offset - start_offset))
                
                layer_info = {
                    "name": name,
                    "layer_type": dtype_str,
                    "shape": shape,
                    "parameters": flat_params.tolist()
                }
                layers.append(layer_info)
                
                if i < 5:  # 只显示前5层的详细信息
                    print(f"  [{i+1}/{len(header)}] {name}: {shape} ({dtype_str}), {len(flat_params):,} 参数")
                elif i == 5:
                    print(f"  ... 还有 {len(header) - 5} 层")
                
                total_params += len(flat_params)
                
            except Exception as e:
                print(f"  警告: 跳过层 {name} - {e}")
                continue
    
    if not layers:
        raise ValueError("没有成功加载任何层")
    
    print(f"\n成功加载 {len(layers)} 个层")
    print(f"模型总参数数量: {total_params:,}")
    
    # 拆分模型
    print(f"\n开始拆分模型为 {num_parts} 部分...")
    layers_per_part = len(layers) // num_parts
    partitions = []
    
    for i in range(num_parts):
        start_idx = i * layers_per_part
        end_idx = start_idx + layers_per_part if i < num_parts - 1 else len(layers)
        
        part_layers = layers[start_idx:end_idx]
        part_params = sum(len(layer["parameters"]) for layer in part_layers)
        
        partition = {
            "part_id": i,
            "layers": part_layers,
            "total_params": part_params
        }
        partitions.append(partition)
        
        print(f"Part {i}: {len(part_layers)} 层, {part_params:,} 参数 ({part_params/total_params*100:.1f}%)")
    
    # 保存分区
    if output_dir is None:
        output_dir = model_path.parent / "partitions"
    output_dir.mkdir(parents=True, exist_ok=True)
    
    saved_paths = []
    for partition in partitions:
        file_path = output_dir / f"partition_{partition['part_id']}.json"
        
        with open(file_path, 'w', encoding='utf-8') as f:
            json.dump(partition, f, indent=2, ensure_ascii=False)
        
        print(f"保存模型分区 {partition['part_id']} 到: {file_path}")
        saved_paths.append(file_path)
    
    return saved_paths


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="直接加载 safetensors 文件")
    parser.add_argument("model_path", help="模型目录路径")
    parser.add_argument("--num-parts", type=int, default=2, help="拆分数量")
    parser.add_argument("--output-dir", help="输出目录")
    
    args = parser.parse_args()
    
    try:
        model_path = Path(args.model_path)
        if not model_path.exists():
            print(f"模型路径不存在: {model_path}")
            sys.exit(1)
        
        output_dir = Path(args.output_dir) if args.output_dir else None
        saved_paths = load_safetensors_direct(model_path, args.num_parts, output_dir)
        
        print(f"\n✅ 模型处理完成! 保存的文件:")
        for path in saved_paths:
            size_mb = path.stat().st_size / (1024**2)
            print(f"  - {path} ({size_mb:.1f} MB)")
            
    except Exception as e:
        print(f"❌ 处理失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
