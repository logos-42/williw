#!/usr/bin/env python3
"""
PyTorch 模型转换工具
将 PyTorch .pt/.pth 文件转换为 .npy 格式，供 Rust GGB 节点使用

支持：
- state_dict 格式（仅参数）
- 完整模型格式（包含模型结构）
- 自动扁平化多层参数为单一向量
"""

import argparse
import numpy as np
import torch
import sys
import os
from pathlib import Path


def flatten_state_dict(state_dict):
    """
    将 PyTorch state_dict 扁平化为单一向量
    
    Args:
        state_dict: PyTorch 模型的 state_dict
        
    Returns:
        numpy array: 扁平化的参数向量
    """
    params = []
    for key, tensor in state_dict.items():
        # 跳过非参数项（如 running_mean, running_var 等）
        if 'running' in key or 'num_batches' in key:
            continue
        # 将张量展平并添加到参数列表
        params.append(tensor.detach().cpu().numpy().flatten())
    
    if not params:
        raise ValueError("未找到可用的模型参数")
    
    # 拼接所有参数
    flattened = np.concatenate(params).astype(np.float32)
    return flattened


def convert_pytorch_to_npy(input_path, output_path, flatten=True):
    """
    将 PyTorch 模型转换为 .npy 格式
    
    Args:
        input_path: PyTorch 模型文件路径 (.pt 或 .pth)
        output_path: 输出 .npy 文件路径
        flatten: 是否扁平化为单一向量（默认 True）
    """
    print(f"加载 PyTorch 模型: {input_path}")
    
    try:
        # 尝试加载为 state_dict
        try:
            checkpoint = torch.load(input_path, map_location='cpu')
            
            # 检查是否是 state_dict
            if isinstance(checkpoint, dict):
                if 'state_dict' in checkpoint:
                    # 包含 'state_dict' 键的完整 checkpoint
                    state_dict = checkpoint['state_dict']
                    print("检测到完整 checkpoint 格式（包含 state_dict）")
                elif all(isinstance(v, torch.Tensor) for v in checkpoint.values()):
                    # 纯 state_dict
                    state_dict = checkpoint
                    print("检测到 state_dict 格式")
                else:
                    # 可能是其他格式，尝试直接使用
                    print("警告: 检测到未知格式，尝试直接处理...")
                    state_dict = checkpoint
            else:
                # 可能是完整模型对象
                if hasattr(checkpoint, 'state_dict'):
                    state_dict = checkpoint.state_dict()
                    print("检测到完整模型对象")
                else:
                    raise ValueError("无法识别的模型格式")
            
            # 扁平化参数
            if flatten:
                params = flatten_state_dict(state_dict)
                print(f"参数已扁平化: {params.shape} (总计 {params.size} 个参数)")
            else:
                # 如果不扁平化，保存第一个张量（用于测试）
                first_key = next(iter(state_dict.keys()))
                params = state_dict[first_key].detach().cpu().numpy().flatten().astype(np.float32)
                print(f"使用第一个参数层: {first_key}, 形状: {params.shape}")
            
            # 保存为 .npy
            output_dir = os.path.dirname(output_path)
            if output_dir and not os.path.exists(output_dir):
                os.makedirs(output_dir, exist_ok=True)
            
            np.save(output_path, params)
            print(f"已保存为 .npy 格式: {output_path}")
            print(f"参数统计:")
            print(f"  - 维度: {params.size}")
            print(f"  - 范围: [{params.min():.6f}, {params.max():.6f}]")
            print(f"  - 均值: {params.mean():.6f}")
            print(f"  - 标准差: {params.std():.6f}")
            
        except Exception as e:
            print(f"错误: 无法加载 PyTorch 模型: {e}")
            sys.exit(1)
            
    except FileNotFoundError:
        print(f"错误: 文件不存在: {input_path}")
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(
        description='将 PyTorch 模型转换为 .npy 格式',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 基本用法
  python convert_pytorch_model.py model.pt model.npy
  
  # 指定输出目录
  python convert_pytorch_model.py model.pt output/model.npy
  
  # 不扁平化（仅用于测试）
  python convert_pytorch_model.py model.pt model.npy --no-flatten
        """
    )
    
    parser.add_argument(
        'input',
        type=str,
        help='输入的 PyTorch 模型文件路径 (.pt 或 .pth)'
    )
    
    parser.add_argument(
        'output',
        type=str,
        nargs='?',
        default=None,
        help='输出的 .npy 文件路径（默认：输入文件名 + .npy）'
    )
    
    parser.add_argument(
        '--no-flatten',
        action='store_true',
        help='不扁平化参数（仅用于测试，不推荐）'
    )
    
    args = parser.parse_args()
    
    # 确定输出路径
    if args.output is None:
        input_path = Path(args.input)
        output_path = input_path.with_suffix('.npy')
    else:
        output_path = Path(args.output)
    
    # 检查输入文件
    if not os.path.exists(args.input):
        print(f"错误: 输入文件不存在: {args.input}")
        sys.exit(1)
    
    # 执行转换
    convert_pytorch_to_npy(
        args.input,
        str(output_path),
        flatten=not args.no_flatten
    )


if __name__ == '__main__':
    main()

