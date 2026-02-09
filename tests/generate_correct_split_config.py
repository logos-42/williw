#!/usr/bin/env python3
"""
生成正确的模型切分配置
基于实际的模型层名称
"""

import json
import sys
from pathlib import Path
from transformers import AutoModelForCausalLM

MODEL_PATH = Path("test_models/models--LiquidAI--LFM2.5-1.2B-Thinking/snapshots/1c9725ba97f047b37bcf53e44e9133ccf1f79333")
OUTPUT_DIR = Path("test_models/test_models/simple_split")

print("=" * 70)
print("🔍 分析模型结构")
print("=" * 70)

# 加载模型
print("\n🔄 正在加载模型...")
model = AutoModelForCausalLM.from_pretrained(str(MODEL_PATH), low_cpu_mem_usage=True)
state_dict = model.state_dict()

# 获取所有层名称
all_keys = sorted(state_dict.keys())

print(f"\n✅ 模型加载成功")
print(f"   总层数: {len(all_keys)}")
print(f"   总参数: {model.num_parameters():,}")

# 分析层结构
layers_by_type = {}
for key in all_keys:
    layer_type = key.split('.')[-2] if '.' in key else 'root'
    if layer_type not in layers_by_type:
        layers_by_type[layer_type] = []
    layers_by_type[layer_type].append(key)

print("\n📊 按类型统计:")
for layer_type, keys in sorted(layers_by_type.items()):
    print(f"   {layer_type}: {len(keys)} 层")

# 设计切分方案
print("\n" + "=" * 70)
print("📋 设计切分方案")
print("=" * 70)

# Node 001: 嵌入层 + 前5层的卷积部分
node_001_layers = [
    "model.embed_tokens.weight",
    "model.embedding_norm.weight",
]
for i in range(5):
    node_001_layers.append(f"model.layers.{i}.conv.conv.weight")
    node_001_layers.append(f"model.layers.{i}.conv.in_proj.weight")
    node_001_layers.append(f"model.layers.{i}.conv.out_proj.weight")

# 计算node_001的参数量
node_001_params = sum(state_dict[k].numel() for k in node_001_layers if k in state_dict)
node_001_size = sum(state_dict[k].numel() * 2 for k in node_001_layers if k in state_dict) / (1024 * 1024)

print(f"\n📦 节点 001:")
print(f"   层数: {len(node_001_layers)}")
print(f"   参数量: {node_001_params:,}")
print(f"   大小: {node_001_size:.2f} MB")

# Node 002: 前5层的FFN部分
node_002_layers = []
for i in range(5):
    node_002_layers.append(f"model.layers.{i}.feed_forward.w1.weight")
    node_002_layers.append(f"model.layers.{i}.feed_forward.w2.weight")
    node_002_layers.append(f"model.layers.{i}.feed_forward.w3.weight")
    node_002_layers.append(f"model.layers.{i}.ffn_norm.weight")
    node_002_layers.append(f"model.layers.{i}.operator_norm.weight")

# 计算node_002的参数量
node_002_params = sum(state_dict[k].numel() for k in node_002_layers if k in state_dict)
node_002_size = sum(state_dict[k].numel() * 2 for k in node_002_layers if k in state_dict) / (1024 * 1024)

print(f"\n📦 节点 002:")
print(f"   层数: {len(node_002_layers)}")
print(f"   参数量: {node_002_params:,}")
print(f"   大小: {node_002_size:.2f} MB")

# Node 003: 第6-10层的self_attn部分
node_003_layers = []
for i in range(6, 11):
    node_003_layers.append(f"model.layers.{i}.self_attn.q_proj.weight")
    node_003_layers.append(f"model.layers.{i}.self_attn.k_proj.weight")
    node_003_layers.append(f"model.layers.{i}.self_attn.v_proj.weight")
    node_003_layers.append(f"model.layers.{i}.self_attn.out_proj.weight")
    node_003_layers.append(f"model.layers.{i}.self_attn.q_layernorm.weight")
    node_003_layers.append(f"model.layers.{i}.self_attn.k_layernorm.weight")

# 计算node_003的参数量
node_003_params = sum(state_dict[k].numel() for k in node_003_layers if k in state_dict)
node_003_size = sum(state_dict[k].numel() * 2 for k in node_003_layers if k in state_dict) / (1024 * 1024)

print(f"\n📦 节点 003:")
print(f"   层数: {len(node_003_layers)}")
print(f"   参数量: {node_003_params:,}")
print(f"   大小: {node_003_size:.2f} MB")

# 生成配置文件
configs = [
    {
        "node_id": "node_001",
        "layer_names": node_001_layers,
        "total_params": node_001_params,
        "estimated_size_mb": node_001_size
    },
    {
        "node_id": "node_002",
        "layer_names": node_002_layers,
        "total_params": node_002_params,
        "estimated_size_mb": node_002_size
    },
    {
        "node_id": "node_003",
        "layer_names": node_003_layers,
        "total_params": node_003_params,
        "estimated_size_mb": node_003_size
    }
]

# 保存配置
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

for config in configs:
    config_file = OUTPUT_DIR / f"{config['node_id']}.json"
    with open(config_file, 'w') as f:
        json.dump(config, f, indent=2)
    print(f"\n✅ 配置已保存: {config_file}")

# 生成汇总
total_params = sum(c["total_params"] for c in configs)
total_size = sum(c["estimated_size_mb"] for c in configs)

summary = {
    "model_name": "LiquidAI/LFM2.5-1.2B-Thinking",
    "total_model_params": model.num_parameters(),
    "split_nodes": len(configs),
    "split_params": total_params,
    "split_size_mb": total_size,
    "coverage": f"{(total_params / model.num_parameters() * 100):.2f}%"
}

summary_file = OUTPUT_DIR / "split_summary.json"
with open(summary_file, 'w') as f:
    json.dump(summary, f, indent=2)

print("\n" + "=" * 70)
print("✅ 切分方案生成完成")
print("=" * 70)
print(f"\n📊 统计信息:")
print(f"   总节点数: {len(configs)}")
print(f"   切分参数: {total_params:,}")
print(f"   切分大小: {total_size:.2f} MB")
print(f"   覆盖率: {summary['coverage']}")
print(f"\n📁 配置目录: {OUTPUT_DIR}")
print("=" * 70)
