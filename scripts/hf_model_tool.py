#!/usr/bin/env python3
"""
HuggingFace Model Downloader
AI Agent 可通过 BashTool 调用此脚本下载模型
支持断点续传、分块下载、实时进度
"""

import os
import sys
import json
import argparse
import time
from pathlib import Path

def download_model(model_name: str, target_path: str, revision: str = "main", show_progress: bool = True) -> dict:
    """从 HuggingFace 下载模型 - 支持断点续传"""
    try:
        from huggingface_hub import snapshot_download, hf_hub_download
    except ImportError:
        return {
            "success": False,
            "error": "huggingface_hub 未安装，请运行: pip install huggingface_hub"
        }
    
    try:
        # 检查目标路径是否已有部分文件（断点续传检测）
        existing_files = []
        if os.path.exists(target_path):
            existing_files = [f for f in os.listdir(target_path) if os.path.isfile(os.path.join(target_path, f))]
        
        resume_mode = len(existing_files) > 0
        
        if show_progress:
            if resume_mode:
                print(f"📂 检测到已有 {len(existing_files)} 个文件，尝试断点续传...")
            else:
                print(f"📥 开始下载模型: {model_name}")
        
        # 下载模型到指定目录（huggingface_hub 默认支持断点续传）
        local_path = snapshot_download(
            repo_id=model_name,
            local_dir=target_path,
            revision=revision,
            resume_download=True,  # 启用断点续传
            ignore_patterns=["*.msgpack", "*.h5", "*.ot"],  # 忽略不需要的文件
        )
        
        # 计算下载的模型大小
        total_size = 0
        file_count = 0
        for root, dirs, files in os.walk(local_path):
            for f in files:
                fp = os.path.join(root, f)
                try:
                    total_size += os.path.getsize(fp)
                    file_count += 1
                except:
                    pass
        
        return {
            "success": True,
            "model_name": model_name,
            "local_path": local_path,
            "size_bytes": total_size,
            "size_mb": round(total_size / (1024 * 1024), 2),
            "size_gb": round(total_size / (1024 * 1024 * 1024), 2),
            "file_count": file_count,
            "resume_mode": resume_mode,
            "message": f"模型 {model_name} 下载完成 ({round(total_size / (1024 * 1024 * 1024), 2)} GB)"
        }
        
    except Exception as e:
        error_msg = str(e)
        # 检查是否是网络中断（可恢复）
        recoverable = any(x in error_msg.lower() for x in ['connection', 'timeout', 'network'])
        return {
            "success": False,
            "error": error_msg,
            "recoverable": recoverable,
            "model_name": model_name
        }


def download_model_with_progress(model_name: str, target_path: str, revision: str = "main") -> dict:
    """带实时进度的模型下载（分块下载）"""
    try:
        from huggingface_hub import hf_hub_download, get_hf_file_metadata
        from huggingface_hub.utils import tqdm
    except ImportError as e:
        return {"success": False, "error": f"需要安装 huggingface_hub: {e}"}
    
    try:
        os.makedirs(target_path, exist_ok=True)
        
        # 获取模型文件列表
        from huggingface_hub import list_repo_files
        files = list(list_repo_files(model_name, revision=revision))
        
        # 过滤模型文件
        model_files = [f for f in files if f.endswith(('.safetensors', '.bin', '.pt', '.pth'))]
        
        if not model_files:
            # 使用完整下载
            return download_model(model_name, target_path, revision, show_progress=True)
        
        print(f"📦 模型包含 {len(model_files)} 个权重文件，开始分块下载...")
        
        downloaded = 0
        total_size = 0
        
        for i, file_path in enumerate(model_files):
            print(f"  [{i+1}/{len(model_files)}] 下载 {file_path}...")
            
            try:
                # 下载单个文件
                dest_path = hf_hub_download(
                    repo_id=model_name,
                    filename=file_path,
                    revision=revision,
                    local_dir=target_path,
                    resume_download=True,
                )
                
                file_size = os.path.getsize(dest_path)
                total_size += file_size
                downloaded += 1
                
                print(f"    ✅ {file_path} ({round(file_size / 1024 / 1024, 1)} MB)")
                
            except Exception as e:
                print(f"    ⚠️ {file_path} 下载失败: {e}")
                continue
        
        if downloaded == 0:
            return {"success": False, "error": "所有文件下载失败"}
        
        return {
            "success": True,
            "model_name": model_name,
            "local_path": target_path,
            "downloaded_files": downloaded,
            "total_files": len(model_files),
            "size_bytes": total_size,
            "size_gb": round(total_size / (1024 * 1024 * 1024), 2),
            "message": f"下载完成: {downloaded}/{len(model_files)} 文件"
        }
        
    except Exception as e:
        return {"success": False, "error": str(e)}


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
    parser.add_argument("operation", choices=["download", "split", "infer", "download-progressive"], help="操作类型")
    parser.add_argument("--model", "-m", help="模型名称或路径")
    parser.add_argument("--output", "-o", help="输出路径")
    parser.add_argument("--shards", "-n", type=int, default=2, help="分片数量")
    parser.add_argument("--input", "-i", help="推理输入文本")
    parser.add_argument("--max-tokens", type=int, default=512, help="最大生成 token 数")
    parser.add_argument("--temperature", type=float, default=0.7, help="温度参数")
    parser.add_argument("--revision", default="main", help="模型版本/分支")
    
    args = parser.parse_args()
    
    result = {}
    
    if args.operation == "download":
        if not args.model or not args.output:
            print(json.dumps({"success": False, "error": "需要 --model 和 --output 参数"}))
            sys.exit(1)
        result = download_model(args.model, args.output, args.revision, show_progress=True)
        
    elif args.operation == "download-progressive":
        # 带实时进度的分块下载
        if not args.model or not args.output:
            print(json.dumps({"success": False, "error": "需要 --model 和 --output 参数"}))
            sys.exit(1)
        result = download_model_with_progress(args.model, args.output, args.revision)
        
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
