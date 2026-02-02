#!/usr/bin/env python3
"""
Python 脚本：生成元数据
被 Rust 模块调用
"""
import json
import sys
import argparse
import torch
from transformers import AutoModel
from typing import Dict, Any

# 算力系数
OPERATION_COSTS = {
    'conv2d': 2.0,
    'linear': 2.0,
    'attention': 4.0,
    'layernorm': 1.0,
    'embedding': 0.5,
    'activation': 0.1,
    'pooling': 0.2,
}

def identify_layer_type(layer_name: str, tensor: torch.Tensor) -> str:
    """识别层类型"""
    name_lower = layer_name.lower()
    if 'conv' in name_lower or 'conv2d' in name_lower:
        return 'conv2d'
    elif 'attention' in name_lower or 'attn' in name_lower:
        return 'attention'
    elif 'embedding' in name_lower or 'emb' in name_lower:
        return 'embedding'
    elif 'norm' in name_lower or 'ln' in name_lower or 'bn' in name_lower:
        return 'layernorm'
    elif 'weight' in name_lower and len(tensor.shape) == 2:
        return 'linear'
    else:
        return 'linear'

def estimate_layer_compute(layer_name: str, tensor: torch.Tensor, 
                          model_type: str, batch_size: int, sequence_length: int) -> float:
    """估算单层算力需求"""
    num_params = tensor.numel()
    layer_type = identify_layer_type(layer_name, tensor)
    cost_per_param = OPERATION_COSTS.get(layer_type, 2.0)
    layer_compute = num_params * cost_per_param * batch_size
    
    if model_type == "transformer":
        layer_compute *= (sequence_length / 512.0)
    
    return layer_compute

def generate_metadata(model_name: str, model_path: str, 
                     batch_size: int, sequence_length: int) -> Dict[str, Any]:
    """生成元数据"""
    # 加载模型
    model = AutoModel.from_pretrained(model_name, cache_dir=model_path)
    state_dict = model.state_dict()
    
    # 检测模型类型
    model_type = "transformer" if any(
        "transformer" in k.lower() or "attention" in k.lower() 
        for k in state_dict.keys()
    ) else "mlp"
    
    # 提取元数据
    metadata = {
        "model_name": model_name,
        "model_type": model_type,
        "batch_size": batch_size,
        "sequence_length": sequence_length,
        "layers": []
    }
    
    total_compute = 0.0
    
    for name, tensor in state_dict.items():
        layer_compute = estimate_layer_compute(name, tensor, model_type, batch_size, sequence_length)
        total_compute += layer_compute
        
        metadata["layers"].append({
            "name": name,
            "shape": list(tensor.shape),
            "num_params": tensor.numel(),
            "compute_required": layer_compute,
            "layer_type": identify_layer_type(name, tensor),
            "dtype": str(tensor.dtype)
        })
    
    metadata["total_compute"] = total_compute
    metadata["total_layers"] = len(metadata["layers"])
    
    return metadata

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-name", required=True)
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--sequence-length", type=int, default=512)
    
    args = parser.parse_args()
    
    metadata = generate_metadata(
        args.model_name,
        args.model_path,
        args.batch_size,
        args.sequence_length
    )
    
    # 输出 JSON
    print(json.dumps(metadata, indent=2))
