#!/usr/bin/env python3
"""
HuggingFace Model Downloader
AI Agent 可通过 BashTool 调用此脚本下载模型
"""

import os
import sys
import json
import argparse
from pathlib import Path

def download_model(model_name: str, target_path: str, revision: str = "main") -> dict:
    """从 HuggingFace 下载模型"""
    try:
        from huggingface_hub import snapshot_download
    except ImportError:
        return {
            "success": False,
            "error": "huggingface_hub 未安装，请运行: pip install huggingface_hub"
        }
    
    try:
        # 下载模型到指定目录
        local_path = snapshot_download(
            repo_id=model_name,
            local_dir=target_path,
            revision=revision,
            resume_download=True,
        )
        
        # 计算下载的模型大小
        total_size = 0
        for root, dirs, files in os.walk(local_path):
            for f in files:
                fp = os.path.join(root, f)
                total_size += os.path.getsize(fp)
        
        return {
            "success": True,
            "model_name": model_name,
            "local_path": local_path,
            "size_bytes": total_size,
            "size_mb": round(total_size / (1024 * 1024), 2),
            "message": f"模型 {model_name} 下载成功"
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "model_name": model_name
        }


def split_model_by_layers(model_path: str, output_dir: str, num_shards: int) -> dict:
    """按层切分模型（简化版）"""
    import shutil
    import hashlib
    
    os.makedirs(output_dir, exist_ok=True)
    
    # 查找模型文件
    model_path = Path(model_path)
    if not model_path.exists():
        return {"success": False, "error": f"模型路径不存在: {model_path}"}
    
    # 如果是目录，查找 safetensors 或 bin 文件
    if model_path.is_dir():
        # 查找主要的模型文件
        safetensors_files = list(model_path.glob("*.safetensors"))
        bin_files = list(model_path.glob("*.bin"))
        
        if safetensors_files:
            model_files = safetensors_files
        elif bin_files:
            model_files = bin_files
        else:
            return {"success": False, "error": "未找到模型文件"}
        
        # 简单切分：按文件数量平均分配
        files_per_shard = max(1, len(model_files) // num_shards)
        
        shards_info = []
        for i in range(num_shards):
            shard_dir = os.path.join(output_dir, f"shard_{i}")
            os.makedirs(shard_dir, exist_ok=True)
            
            start_idx = i * files_per_shard
            end_idx = min((i + 1) * files_per_shard, len(model_files))
            
            for mf in model_files[start_idx:end_idx]:
                dst = os.path.join(shard_dir, mf.name)
                shutil.copy2(mf, dst)
            
            # 计算校验和
            sha256 = hashlib.sha256()
            for mf in model_files[start_idx:end_idx]:
                with open(mf, 'rb') as f:
                    for chunk in iter(lambda: f.read(4096), b""):
                        sha256.update(chunk)
            
            shards_info.append({
                "shard_id": f"shard_{i}",
                "node_id": f"node_{i}",
                "files": [str(mf.name) for mf in model_files[start_idx:end_idx]],
                "checksum": sha256.hexdigest()
            })
    else:
        # 单一文件切分
        file_size = os.path.getsize(model_path)
        chunk_size = file_size // num_shards
        
        shards_info = []
        with open(model_path, 'rb') as f:
            for i in range(num_shards):
                shard_path = os.path.join(output_dir, f"shard_{i}.bin")
                chunk = f.read(chunk_size)
                with open(shard_path, 'wb') as sf:
                    sf.write(chunk)
                
                # 计算校验和
                sha256 = hashlib.sha256()
                sha256.update(chunk)
                
                shards_info.append({
                    "shard_id": f"shard_{i}",
                    "node_id": f"node_{i}",
                    "files": [f"shard_{i}.bin"],
                    "checksum": sha256.hexdigest()
                })
    
    return {
        "success": True,
        "model_path": str(model_path),
        "num_shards": num_shards,
        "shards": shards_info,
        "output_dir": output_dir
    }


def run_inference(model_path: str, input_text: str, max_tokens: int = 512, temperature: float = 0.7) -> dict:
    """运行本地推理"""
    try:
        import torch
        from transformers import AutoTokenizer, AutoModelForCausalLM
    except ImportError:
        return {
            "success": False,
            "error": "transformers/torch 未安装，请运行: pip install transformers torch"
        }
    
    try:
        # 加载模型
        tokenizer = AutoTokenizer.from_pretrained(model_path)
        model = AutoModelForCausalLM.from_pretrained(
            model_path,
            torch_dtype=torch.float16,
            device_map="auto"
        )
        
        # 运行推理
        inputs = tokenizer(input_text, return_tensors="pt").to(model.device)
        
        outputs = model.generate(
            **inputs,
            max_new_tokens=max_tokens,
            temperature=temperature,
            do_sample=True
        )
        
        result = tokenizer.decode(outputs[0], skip_special_tokens=True)
        
        return {
            "success": True,
            "input": input_text,
            "output": result,
            "model": model_path
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }


def main():
    parser = argparse.ArgumentParser(description="HuggingFace 模型处理工具")
    parser.add_argument("operation", choices=["download", "split", "infer"], help="操作类型")
    parser.add_argument("--model", "-m", help="模型名称或路径")
    parser.add_argument("--output", "-o", help="输出路径")
    parser.add_argument("--shards", "-n", type=int, default=2, help="分片数量")
    parser.add_argument("--input", "-i", help="推理输入文本")
    parser.add_argument("--max-tokens", type=int, default=512, help="最大生成 token 数")
    parser.add_argument("--temperature", type=float, default=0.7, help="温度参数")
    
    args = parser.parse_args()
    
    result = {}
    
    if args.operation == "download":
        if not args.model or not args.output:
            print(json.dumps({"success": False, "error": "需要 --model 和 --output 参数"}))
            sys.exit(1)
        result = download_model(args.model, args.output)
        
    elif args.operation == "split":
        if not args.model or not args.output:
            print(json.dumps({"success": False, "error": "需要 --model 和 --output 参数"}))
            sys.exit(1)
        result = split_model_by_layers(args.model, args.output, args.shards)
        
    elif args.operation == "infer":
        if not args.model or not args.input:
            print(json.dumps({"success": False, "error": "需要 --model 和 --input 参数"}))
            sys.exit(1)
        result = run_inference(args.model, args.input, args.max_tokens, args.temperature)
    
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
