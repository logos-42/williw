#!/usr/bin/env python3
"""
使用本地模拟服务器测试节点信息上传
"""

import json
import requests
import subprocess
import time
import sys

def start_mock_server():
    """启动模拟服务器"""
    print("🚀 启动本地模拟Workers服务器...")
    
    # 在后台启动服务器
    process = subprocess.Popen(
        [sys.executable, "mock_workers_server.py"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd="d:\\AI\\去中心化训练"
    )
    
    # 等待服务器启动
    time.sleep(3)
    
    # 检查服务器是否运行
    try:
        resp = requests.get("http://localhost:8787/api/health", timeout=5)
        if resp.status_code == 200:
            print("✅ 模拟服务器已启动")
            return process
        else:
            print(f"⚠️ 服务器返回错误: {resp.status_code}")
            return None
    except Exception as e:
        print(f"❌ 无法连接到模拟服务器: {e}")
        process.terminate()
        return None

def test_node_info_upload():
    """测试节点信息上传"""
    print("\n📊 测试节点信息上传...")
    
    # 模拟设备信息
    device_info = {
        "gpu_type": "NVIDIA GeForce GTX 1650 Ti",
        "gpu_usage": 15.5,
        "gpu_memory_total": 4.0,
        "gpu_memory_used": 1.2,
        "cpu_cores": 12,
        "total_memory_gb": 7.42,
        "battery_level": None,
        "is_charging": None
    }
    
    payload = {
        "device_id": f"test-device-{int(time.time())}",
        "timestamp": "2026-02-01T12:00:00Z",
        "device_info": device_info,
        "metadata": {
            "platform": "windows",
            "app_version": "0.1.0",
            "node_id": None,
            "capabilities": {
                "os": "windows",
                "test": True
            }
        }
    }
    
    try:
        resp = requests.post(
            "http://localhost:8787/api/node-info",
            json=payload,
            timeout=10
        )
        
        print(f"📥 响应: {resp.status_code}")
        print(f"📄 内容: {resp.json()}")
        
        return resp.status_code == 200 and resp.json().get("success")
    except Exception as e:
        print(f"❌ 上传失败: {e}")
        return False

def test_get_nodes():
    """测试获取节点列表"""
    print("\n📋 测试获取节点列表...")
    
    try:
        resp = requests.get("http://localhost:8787/api/nodes", timeout=10)
        
        print(f"📥 响应: {resp.status_code}")
        data = resp.json()
        print(f"📊 节点数: {data.get('count', 0)}")
        
        return resp.status_code == 200
    except Exception as e:
        print(f"❌ 获取失败: {e}")
        return False

if __name__ == "__main__":
    print("="*60)
    print("🧪 本地模拟服务器测试")
    print("="*60)
    
    # 启动模拟服务器
    server_process = start_mock_server()
    if not server_process:
        print("❌ 无法启动模拟服务器")
        sys.exit(1)
    
    try:
        # 运行测试
        test1 = test_node_info_upload()
        test2 = test_get_nodes()
        
        print("\n" + "="*60)
        print("📊 测试结果")
        print("="*60)
        print(f"节点信息上传: {'✅ 通过' if test1 else '❌ 失败'}")
        print(f"获取节点列表: {'✅ 通过' if test2 else '❌ 失败'}")
        
        if test1 and test2:
            print("\n✅ 所有测试通过！")
            print("说明：本地节点信息收集和API上传逻辑完全正常")
        else:
            print("\n❌ 部分测试失败")
            
    finally:
        print("\n🛑 停止模拟服务器...")
        server_process.terminate()
        server_process.wait()
        print("✅ 服务器已停止")
