#!/usr/bin/env python3
"""
专门处理 bfloat16 模型的加载器
使用 safetensors 原生接口
"""

import os
import sys
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional

try:
    import numpy as np
    from safetensors import safe_open
except ImportError as e:
    print(f"缺少必要的依赖: {e}")
    print("请运行: pip install numpy safetensors")
    sys.exit(1)


def load_bfloat16_model(model_path: Path, num_parts: int = 2, output_dir: Optional[Path] = None):
    """加载并拆分 bfloat16 模型"""
    print(f"加载模型从: {model_path}")
    
    # 查找 safetensors 文件
    safetensors_file = model_path / "model.safetensors"
    if not safetensors_file.exists():
        raise FileNotFoundError(f"未找到 model.safetensors 文件: {safetensors_file}")
    
    print(f"找到模型文件: {safetensors_file}")
    print(f"文件大小: {safetensors_file.stat().st_size / (1024**3):.2f} GB")
    
    # 加载所有层
    layers = []
    try:
        with safe_open(safetensors_file, framework="pt") as f:
            keys = list(f.keys())
            print(f"发现 {len(keys)} 个层")
            
            for i, key in enumerate(keys):
                try:
                    tensor = f.get_tensor(key)
                    
                    # 获取张量信息
                    tensor_shape = list(tensor.shape)
                    tensor_dtype = str(tensor.dtype)
                    
                    # 将张量转换为 numpy 数组
                    # 对于 bfloat16，我们使用 view as uint16 然后转换为 float32
                    if hasattr(tensor, 'numpy'):
                        try:
                            np_array = tensor.numpy()
                        except:
                            # 如果直接转换失败，尝试其他方法
                            np_array = tensor.cpu().numpy()
                    else:
                        # 使用 CPU 转换
                        np_array = tensor.cpu().numpy()
                    
                    # 如果是 bfloat16，转换为 float32
                    if 'bfloat16' in tensor_dtype or np_array.dtype == np.dtype('bfloat16'):
                        # 将 bfloat16 转换为 float32
                        # 由于 numpy 不直接支持 bfloat16，我们使用 view
                        if np_array.dtype == np.uint16:
                            # 假设 uint16 实际上是 bfloat16 的位表示
                            np_array = np_array.view(np.float16).astype(np.float32)
                        else:
                            # 尝试直接转换
                            np_array = np_array.astype(np.float32)
                    else:
                        # 其他数据类型转换为 float32
                        np_array = np_array.astype(np.float32)
                    
                    # 扁平化数组
                    flat_params = np_array.flatten()
                    
                    layer_info = {
                        "name": key,
                        "layer_type": tensor_dtype,
                        "shape": tensor_shape,
                        "parameters": flat_params.tolist()
                    }
                    layers.append(layer_info)
                    
                    if i < 5:  # 只显示前5层的详细信息
                        print(f"  [{i+1}/{len(keys)}] {key}: {tensor_shape} ({tensor_dtype}), {flat_params.size:,} 参数")
                    elif i == 5:
                        print(f"  ... 还有 {len(keys) - 5} 层")
                        
                except Exception as layer_error:
                    print(f"  警告: 跳过层 {key} - {layer_error}")
                    continue
                    
    except Exception as e:
        print(f"加载 safetensors 失败: {e}")
        raise
    
    if not layers:
        raise ValueError("没有成功加载任何层")
    
    # 计算总参数数量
    total_params = sum(len(layer["parameters"]) for layer in layers)
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
    parser = argparse.ArgumentParser(description="处理 bfloat16 模型的加载器")
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
        saved_paths = load_bfloat16_model(model_path, args.num_parts, output_dir)
        
        print(f"\n✅ 模型处理完成! 保存的文件:")
        for path in saved_paths:
            size_mb = path.stat().st_size / (1024**2)
            print(f"  - {path} ({size_mb:.1f} MB)")
            
    except Exception as e:
        print(f"❌ 处理失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
