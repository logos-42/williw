#!/usr/bin/env python3
"""
Python 脚本：切分模型
被 Rust 模块调用
"""
import json
import sys
import argparse
import torch
from pathlib import Path
from typing import Dict, Any

def split_model(model_name: str, model_path: str,
               plan_file: str, output_dir: str, node_id: str) -> Dict[str, Any]:
    """根据方案切分模型"""
    print(f"开始切分模型: {node_id}", file=sys.stderr)
    print(f"  模型名称: {model_name}", file=sys.stderr)
    print(f"  模型路径: {model_path}", file=sys.stderr)

    # 加载切分方案
    with open(plan_file, 'r') as f:
        plan = json.load(f)

    layer_names = plan["layer_names"]
    print(f"  需要提取 {len(layer_names)} 个层", file=sys.stderr)

    # 检查模型路径
    model_path_obj = Path(model_path)
    if not model_path_obj.exists():
        # 如果直接路径不存在，尝试从缓存加载
        print(f"  模型路径不存在，尝试从缓存加载...", file=sys.stderr)

    # 尝试加载模型
    try:
        # 方法1: 直接从 safetensors 加载
        from safetensors.torch import load_file

        safetensors_path = model_path_obj / "model.safetensors"
        if safetensors_path.exists():
            print(f"  从 safetensors 加载: {safetensors_path}", file=sys.stderr)
            state_dict = load_file(str(safetensors_path))
        else:
            # 方法2: 使用 transformers 加载
            print(f"  使用 transformers 加载模型", file=sys.stderr)
            from transformers import AutoModel
            model = AutoModel.from_pretrained(
                str(model_path_obj),
                trust_remote_code=True,
                torch_dtype=torch.float16
            )
            state_dict = model.state_dict()

        print(f"  成功加载模型，共 {len(state_dict)} 个张量", file=sys.stderr)

    except Exception as e:
        print(f"  加载模型失败: {str(e)}", file=sys.stderr)
        raise

    # 提取本节点的层
    my_shard = {}
    missing_layers = []
    for layer_name in layer_names:
        if layer_name in state_dict:
            my_shard[layer_name] = state_dict[layer_name]
            print(f"    ✓ {layer_name}", file=sys.stderr)
        else:
            missing_layers.append(layer_name)
            print(f"    ✗ {layer_name} (未找到)", file=sys.stderr)

    if missing_layers:
        print(f"  警告: {len(missing_layers)} 个层未找到", file=sys.stderr)
        if len(missing_layers) > len(layer_names) / 2:
            raise ValueError(f"太多层未找到: {missing_layers[:5]}...")

    # 保存分片
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    shard_path = output_path / f"shard_{node_id}.safetensors"
    print(f"  保存分片到: {shard_path}", file=sys.stderr)

    # 使用 safetensors 格式保存
    from safetensors.torch import save_file
    save_file(my_shard, str(shard_path))

    # 计算统计信息
    total_params = sum(p.numel() for p in my_shard.values())
    # float16 = 2 bytes per parameter
    shard_size_mb = sum(p.numel() * 2 for p in my_shard.values()) / (1024 * 1024)

    result = {
        "node_id": node_id,
        "shard_path": str(shard_path),
        "layer_names": layer_names,
        "total_params": total_params,
        "shard_size_mb": shard_size_mb
    }

    print(f"  切分完成: {total_params:,} 参数, {shard_size_mb:.2f} MB", file=sys.stderr)

    return result

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-name", required=True)
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--plan-file", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--node-id", required=True)

    args = parser.parse_args()

    try:
        result = split_model(
            args.model_name,
            args.model_path,
            args.plan_file,
            args.output_dir,
            args.node_id
        )

        # 输出 JSON
        print(json.dumps(result, indent=2))
        sys.exit(0)

    except Exception as e:
        print(f"ERROR: {str(e)}", file=sys.stderr)
        sys.exit(1)
