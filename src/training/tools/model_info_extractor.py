#!/usr/bin/env python3
"""
模型信息提取器
无需加载权重，只提取模型元数据和结构信息
"""

import os
import sys
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional


def extract_model_info(model_path: Path) -> Dict[str, Any]:
    """提取模型基本信息"""
    print(f"提取模型信息从: {model_path}")
    
    if not model_path.exists():
        raise FileNotFoundError(f"模型路径不存在: {model_path}")
    
    # 读取配置文件
    config_file = model_path / "config.json"
    if not config_file.exists():
        raise FileNotFoundError(f"未找到 config.json 文件: {config_file}")
    
    with open(config_file, 'r', encoding='utf-8') as f:
        config = json.load(f)
    
    # 检查 safetensors 文件
    safetensors_file = model_path / "model.safetensors"
    file_size = 0
    if safetensors_file.exists():
        file_size = safetensors_file.stat().st_size
    
    # 计算模型参数数量（基于配置估算）
    hidden_size = config.get('hidden_size', 0)
    num_hidden_layers = config.get('num_hidden_layers', 0)
    vocab_size = config.get('vocab_size', 0)
    intermediate_size = config.get('intermediate_size', 0)
    
    # 简化的参数估算
    embedding_params = vocab_size * hidden_size
    attention_params = num_hidden_layers * hidden_size * hidden_size * 3  # Q, K, V
    ffn_params = num_hidden_layers * hidden_size * intermediate_size
    output_params = vocab_size * hidden_size
    
    estimated_params = embedding_params + attention_params + ffn_params + output_params
    
    model_info = {
        "model_name": model_path.name,
        "model_type": config.get('model_type', 'unknown'),
        "architecture": config.get('architectures', ['unknown'])[0],
        "hidden_size": hidden_size,
        "num_layers": num_hidden_layers,
        "num_attention_heads": config.get('num_attention_heads', 0),
        "vocab_size": vocab_size,
        "max_position_embeddings": config.get('max_position_embeddings', 0),
        "dtype": config.get('dtype', 'unknown'),
        "file_size_gb": file_size / (1024**3),
        "estimated_parameters": estimated_params,
        "layer_types": config.get('layer_types', []),
        "config": config
    }
    
    return model_info


def create_model_partitions_info(model_info: Dict[str, Any], num_parts: int = 2) -> List[Dict[str, Any]]:
    """创建模型分区信息（不实际加载权重）"""
    print(f"创建 {num_parts} 个分区的信息...")
    
    num_layers = model_info['num_layers']
    estimated_params = model_info['estimated_parameters']
    
    partitions = []
    layers_per_part = num_layers // num_parts
    
    for i in range(num_parts):
        start_layer = i * layers_per_part
        end_layer = start_layer + layers_per_part if i < num_parts - 1 else num_layers
        
        # 估算每个分区的参数数量
        part_ratio = (end_layer - start_layer) / num_layers
        part_params = int(estimated_params * part_ratio)
        
        # 添加嵌入层到第一个分区
        if i == 0:
            embedding_params = model_info['vocab_size'] * model_info['hidden_size']
            part_params += embedding_params
        
        # 添加输出层到最后一个分区
        if i == num_parts - 1:
            output_params = model_info['vocab_size'] * model_info['hidden_size']
            part_params += output_params
        
        partition_info = {
            "part_id": i,
            "layer_range": [start_layer, end_layer],
            "num_layers": end_layer - start_layer,
            "estimated_params": part_params,
            "estimated_size_gb": (part_params * 4) / (1024**3),  # 假设 float32
            "description": f"Layers {start_layer}-{end_layer-1}"
        }
        
        partitions.append(partition_info)
    
    return partitions


def save_model_info(model_info: Dict[str, Any], partitions: List[Dict[str, Any]], output_dir: Path):
    """保存模型信息和分区信息"""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 保存模型信息
    info_file = output_dir / "model_info.json"
    with open(info_file, 'w', encoding='utf-8') as f:
        json.dump(model_info, f, indent=2, ensure_ascii=False)
    print(f"保存模型信息到: {info_file}")
    
    # 保存分区信息
    partitions_file = output_dir / "partitions_info.json"
    with open(partitions_file, 'w', encoding='utf-8') as f:
        json.dump({
            "num_partitions": len(partitions),
            "partitions": partitions
        }, f, indent=2, ensure_ascii=False)
    print(f"保存分区信息到: {partitions_file}")
    
    # 创建分区占位文件（用于后续实际权重加载）
    for partition in partitions:
        placeholder_file = output_dir / f"partition_{partition['part_id']}_placeholder.json"
        placeholder_data = {
            "part_id": partition["part_id"],
            "layer_range": partition["layer_range"],
            "estimated_params": partition["estimated_params"],
            "status": "placeholder - needs GPU loading",
            "note": "This is a placeholder. Actual weights need to be loaded with GPU support."
        }
        
        with open(placeholder_file, 'w', encoding='utf-8') as f:
            json.dump(placeholder_data, f, indent=2, ensure_ascii=False)
        print(f"创建分区占位文件: {placeholder_file}")
    
    return [info_file, partitions_file] + [output_dir / f"partition_{i['part_id']}_placeholder.json" for i in partitions]


def create_gpu_loading_script(output_dir: Path, model_path: Path):
    """创建 GPU 环境下的权重加载脚本"""
    script_content = '''#!/usr/bin/env python3
"""
GPU 环境下的模型权重加载脚本
在有 GPU 支持的环境中运行此脚本来实际加载权重
"""

import torch
import numpy as np
from safetensors import safe_open
import json
from pathlib import Path

def load_model_weights_gpu(model_path: Path, output_dir: Path, num_parts: int = 2):
    """在 GPU 环境中加载模型权重"""
    print("在 GPU 环境中加载模型权重...")
    
    # 检查 CUDA 可用性
    if not torch.cuda.is_available():
        print("警告: 未检测到 CUDA，将使用 CPU")
        device = "cpu"
    else:
        device = "cuda"
        print(f"使用 GPU: {torch.cuda.get_device_name()}")
    
    safetensors_file = model_path / "model.safetensors"
    if not safetensors_file.exists():
        raise FileNotFoundError(f"未找到模型文件: {safetensors_file}")
    
    # 加载所有权重
    weights = {}
    with safe_open(safetensors_file, framework="pt") as f:
        for key in f.keys():
            tensor = f.get_tensor(key)
            weights[key] = tensor.to(device)
            print(f"加载: {key} - {tensor.shape} - {tensor.dtype}")
    
    # 按层拆分权重
    layers = []
    for name, tensor in weights.items():
        # 转换为 float32 并扁平化
        if tensor.dtype == torch.bfloat16:
            tensor = tensor.float()
        
        flat_params = tensor.flatten().cpu().numpy().astype(np.float32)
        
        layer_info = {
            "name": name,
            "layer_type": str(tensor.dtype),
            "shape": list(tensor.shape),
            "parameters": flat_params.tolist()
        }
        layers.append(layer_info)
    
    # 拆分为多个部分
    total_layers = len(layers)
    layers_per_part = total_layers // num_parts
    
    partitions = []
    for i in range(num_parts):
        start_idx = i * layers_per_part
        end_idx = start_idx + layers_per_part if i < num_parts - 1 else total_layers
        
        part_layers = layers[start_idx:end_idx]
        part_params = sum(len(layer["parameters"]) for layer in part_layers)
        
        partition = {
            "part_id": i,
            "layers": part_layers,
            "total_params": part_params
        }
        partitions.append(partition)
        
        # 保存分区
        output_file = output_dir / f"partition_{i}_weights.json"
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(partition, f, indent=2, ensure_ascii=False)
        
        print(f"保存分区 {i}: {len(part_layers)} 层, {part_params:,} 参数")
    
    print("权重加载和拆分完成!")
    return partitions

if __name__ == "__main__":
    model_path = Path("''' + str(model_path) + '''")
    output_dir = Path("''' + str(output_dir) + '''")
    
    load_model_weights_gpu(model_path, output_dir)
'''
    
    script_file = output_dir / "load_weights_gpu.py"
    with open(script_file, 'w', encoding='utf-8') as f:
        f.write(script_content)
    
    print(f"创建 GPU 加载脚本: {script_file}")
    return script_file


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="提取模型信息（无需加载权重）")
    parser.add_argument("model_path", help="模型目录路径")
    parser.add_argument("--num-parts", type=int, default=2, help="分区数量")
    parser.add_argument("--output-dir", help="输出目录")
    
    args = parser.parse_args()
    
    try:
        model_path = Path(args.model_path)
        output_dir = Path(args.output_dir) if args.output_dir else model_path.parent / "model_info"
        
        # 提取模型信息
        model_info = extract_model_info(model_path)
        
        # 创建分区信息
        partitions = create_model_partitions_info(model_info, args.num_parts)
        
        # 保存信息
        saved_files = save_model_info(model_info, partitions, output_dir)
        
        # 创建 GPU 加载脚本
        gpu_script = create_gpu_loading_script(output_dir, model_path)
        
        print(f"\n✅ 模型信息提取完成!")
        print(f"模型类型: {model_info['model_type']}")
        print(f"架构: {model_info['architecture']}")
        print(f"参数数量: {model_info['estimated_parameters']:,}")
        print(f"文件大小: {model_info['file_size_gb']:.2f} GB")
        print(f"数据类型: {model_info['dtype']}")
        
        print(f"\n📁 生成的文件:")
        for file_path in saved_files + [gpu_script]:
            print(f"  - {file_path}")
        
        print(f"\n🚀 下一步:")
        print(f"1. 在有 GPU 支持的环境中运行: python {gpu_script}")
        print(f"2. 这将实际加载权重并创建完整的分区文件")
        
    except Exception as e:
        print(f"❌ 处理失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
