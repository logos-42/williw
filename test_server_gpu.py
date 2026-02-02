#!/usr/bin/env python3
"""
测试GPU服务器
"""

import requests
import json
import time

SERVER_URL = "http://localhost:8000"

def test_health():
    """测试健康检查"""
    print("\n📊 测试健康检查...")
    try:
        resp = requests.get(f"{SERVER_URL}/", timeout=5)
        data = resp.json()
        print(f"   状态: {data.get('status')}")
        print(f"   CUDA可用: {data.get('cuda_available')}")
        print(f"   Transformers可用: {data.get('transformers_available')}")
        return True
    except Exception as e:
        print(f"   ❌ 错误: {e}")
        return False

def test_inference():
    """测试推理"""
    print("\n🧠 测试GPU推理...")
    try:
        payload = {
            "model_path": "local",
            "input_text": "你好，请介绍一下人工智能。",
            "max_length": 100
        }
        
        print(f"   输入: {payload['input_text']}")
        start = time.time()
        resp = requests.post(f"{SERVER_URL}/infer", json=payload, timeout=120)
        elapsed = time.time() - start
        
        data = resp.json()
        print(f"   状态: {data.get('status')}")
        print(f"   消息: {data.get('message')}")
        print(f"   处理时间: {data.get('processing_time', 0):.2f}秒")
        print(f"   实际耗时: {elapsed:.2f}秒")
        print(f"   输出: {data.get('result', '无输出')[:100]}...")
        return data.get('status') == 'success'
    except Exception as e:
        print(f"   ❌ 错误: {e}")
        return False

def test_gpu_utilization():
    """检查GPU利用率"""
    print("\n🎮 检查GPU利用率...")
    try:
        import subprocess
        result = subprocess.run(
            ["nvidia-smi", "--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            gpu_util, mem_used = result.stdout.strip().split(",")
            print(f"   GPU利用率: {gpu_util}")
            print(f"   显存使用: {mem_used}")
        else:
            print(f"   无法获取GPU信息")
    except Exception as e:
        print(f"   跳过GPU检查: {e}")

if __name__ == "__main__":
    print("="*60)
    print("🚀 GPU推理服务器测试")
    print("="*60)
    print(f"服务器地址: {SERVER_URL}")
    
    # 等待服务器启动
    print("\n⏳ 等待服务器启动...")
    for i in range(10):
        try:
            requests.get(f"{SERVER_URL}/", timeout=2)
            print("   ✅ 服务器已就绪")
            break
        except:
            print(f"   等待中... ({i+1}/10)")
            time.sleep(1)
    
    # 运行测试
    test_health()
    test_gpu_utilization()
    success = test_inference()
    test_gpu_utilization()
    
    print("\n" + "="*60)
    if success:
        print("✅ 所有测试通过！GPU推理正常工作")
    else:
        print("❌ 测试失败，请检查服务器日志")
    print("="*60)
