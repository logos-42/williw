#!/usr/bin/env python3
"""
Hugging Face 模型处理助手
用于下载和转换 Llama 3.2 1B 模型
"""

import os
import sys
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional

try:
    import torch
    import numpy as np
    from huggingface_hub import snapshot_download, hf_hub_download
    from safetensors import safe_open
except ImportError as e:
    print(f"缺少必要的依赖: {e}")
    print("请运行: pip install torch numpy huggingface_hub safetensors")
    sys.exit(1)


class HFModelHelper:
    """Hugging Face 模型处理助手"""
    
    def __init__(self, model_id: str, cache_dir: Optional[str] = None):
        self.model_id = model_id
        self.cache_dir = Path(cache_dir) if cache_dir else Path("./models/huggingface")
        self.cache_dir.mkdir(parents=True, exist_ok=True)
    
    def download_model(self, revision: str = "main", token: Optional[str] = None) -> Path:
        """下载模型到本地缓存"""
        print(f"开始下载模型: {self.model_id}")
        
        try:
            model_path = snapshot_download(
                repo_id=self.model_id,
                revision=revision,
                cache_dir=str(self.cache_dir),
                token=token,
                resume_download=True
            )
            print(f"模型下载成功: {model_path}")
            return Path(model_path)
        except Exception as e:
            print(f"下载模型失败: {e}")
            raise
    
    def load_model_layers(self, model_path: Path) -> List[Dict[str, Any]]:
        """加载模型的所有层"""
        print(f"加载模型参数从: {model_path}")
        
        # 查找模型权重文件
        safetensors_file = model_path / "model.safetensors"
        pytorch_file = model_path / "pytorch_model.bin"
        
        if safetensors_file.exists():
            return self._load_safetensors(safetensors_file)
        elif pytorch_file.exists():
            return self._load_pytorch_model(pytorch_file)
        else:
            raise FileNotFoundError("未找到模型权重文件")
    
    def _load_safetensors(self, file_path: Path) -> List[Dict[str, Any]]:
        """从 safetensors 文件加载参数"""
        layers = []
        
        try:
            with safe_open(file_path, framework="pt") as f:
                for key in f.keys():
                    tensor = f.get_tensor(key)
                    
                    # 将张量转换为扁平的一维数组
                    flat_params = tensor.flatten().numpy().astype(np.float32)
                    
                    layer_info = {
                        "name": key,
                        "layer_type": str(tensor.dtype),
                        "shape": list(tensor.shape),
                        "parameters": flat_params.tolist()
                    }
                    layers.append(layer_info)
                    
                    print(f"加载层: {key}, 形状: {tensor.shape}, 参数数量: {tensor.numel()}")
                    
        except Exception as e:
            print(f"加载 safetensors 失败: {e}")
            raise
        
        return layers
    
    def _load_pytorch_model(self, file_path: Path) -> List[Dict[str, Any]]:
        """从 PyTorch 模型文件加载参数"""
        layers = []
        
        try:
            state_dict = torch.load(file_path, map_location="cpu")
            
            for key, tensor in state_dict.items():
                if isinstance(tensor, torch.Tensor):
                    # 将张量转换为扁平的一维数组
                    flat_params = tensor.flatten().numpy().astype(np.float32)
                    
                    layer_info = {
                        "name": key,
                        "layer_type": str(tensor.dtype),
                        "shape": list(tensor.shape),
                        "parameters": flat_params.tolist()
                    }
                    layers.append(layer_info)
                    
                    print(f"加载层: {key}, 形状: {tensor.shape}, 参数数量: {tensor.numel()}")
                    
        except Exception as e:
            print(f"加载 PyTorch 模型失败: {e}")
            raise
        
        return layers
    
    def split_model_layers(self, layers: List[Dict[str, Any]], num_parts: int = 2) -> List[Dict[str, Any]]:
        """将模型层拆分为多个部分"""
        print(f"开始拆分模型为 {num_parts} 部分...")
        
        # 计算总参数数量
        total_params = sum(len(layer["parameters"]) for layer in layers)
        print(f"模型总参数数量: {total_params:,}")
        
        # 按层数量大致平分
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
            
            print(f"Part {i}: {len(part_layers)} 层, {part_params:,} 参数")
        
        return partitions
    
    def save_partitions(self, partitions: List[Dict[str, Any]], output_dir: Path) -> List[Path]:
        """保存拆分后的模型部分"""
        output_dir.mkdir(parents=True, exist_ok=True)
        saved_paths = []
        
        for partition in partitions:
            file_path = output_dir / f"partition_{partition['part_id']}.json"
            
            with open(file_path, 'w', encoding='utf-8') as f:
                json.dump(partition, f, indent=2, ensure_ascii=False)
            
            print(f"保存模型分区 {partition['part_id']} 到: {file_path}")
            saved_paths.append(file_path)
        
        return saved_paths
    
    def process_model(self, num_parts: int = 2, revision: str = "main", 
                     token: Optional[str] = None, output_dir: Optional[Path] = None) -> List[Path]:
        """完整的模型处理流程"""
        # 1. 下载模型
        model_path = self.download_model(revision, token)
        
        # 2. 加载模型层
        layers = self.load_model_layers(model_path)
        
        # 3. 拆分模型
        partitions = self.split_model_layers(layers, num_parts)
        
        # 4. 保存分区
        output_dir = output_dir or self.cache_dir / self.model_id.replace("/", "_")
        saved_paths = self.save_partitions(partitions, output_dir)
        
        return saved_paths


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="Hugging Face 模型处理工具")
    parser.add_argument("--model-id", default="meta-llama/Llama-3.2-1B", 
                       help="Hugging Face 模型 ID")
    parser.add_argument("--cache-dir", help="模型缓存目录")
    parser.add_argument("--output-dir", help="输出目录")
    parser.add_argument("--num-parts", type=int, default=2, help="拆分数量")
    parser.add_argument("--revision", default="main", help="模型版本")
    parser.add_argument("--token", help="Hugging Face 访问令牌")
    
    args = parser.parse_args()
    
    try:
        helper = HFModelHelper(args.model_id, args.cache_dir)
        output_dir = Path(args.output_dir) if args.output_dir else None
        
        saved_paths = helper.process_model(
            num_parts=args.num_parts,
            revision=args.revision,
            token=args.token,
            output_dir=output_dir
        )
        
        print(f"\n模型处理完成! 保存的文件:")
        for path in saved_paths:
            print(f"  - {path}")
            
    except Exception as e:
        print(f"处理失败: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
