#!/usr/bin/env python3
"""
测试完整节点信息上传到Workers后端
包含iroh节点信息和设备信息
"""

import json
import requests
import time
from datetime import datetime

WORKERS_BASE_URL = "https://williw.sirazede725.workers.dev"

def test_full_node_upload():
    """测试上传完整节点信息"""
    print("\n" + "="*60)
    print("📡 测试完整节点信息上传 (包含iroh)")
    print("="*60)
    
    device_id = f"test-device-{int(time.time())}"
    
    # 构建完整的payload
    payload = {
        "device_id": device_id,
        "node_id": device_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "device_info": {
            "gpu_type": "NVIDIA GeForce GTX 1650 Ti",
            "gpu_usage": 15.5,
            "gpu_memory_total": 4.0,
            "gpu_memory_used": 1.2,
            "cpu_cores": 12,
            "total_memory_gb": 7.42,
            "battery_level": None,
            "is_charging": None
        },
        "iroh_node": {
            "node_id": "iroh-node-abc123",
            "is_running": True,
            "tick_counter": 12345,
            "device_capabilities": {
                "max_memory_mb": 8192,
                "cpu_cores": 12,
                "has_gpu": True,
                "network_type": "wifi",
                "battery_level": 85.0,
                "is_charging": True
            },
            "training_stats": {
                "total_ticks": 12345,
                "accuracy": 0.85,
                "loss": 0.15,
                "samples_processed": 100000
            },
            "peers": [
                {
                    "id": "peer-001",
                    "peer_type": "primary",
                    "similarity": 0.92,
                    "geo_affinity": 0.88,
                    "embedding_dim": 768,
                    "position": {
                        "lat": 39.9042,
                        "lon": 116.4074
                    }
                },
                {
                    "id": "peer-002",
                    "peer_type": "backup",
                    "similarity": 0.85,
                    "geo_affinity": 0.75,
                    "embedding_dim": 768,
                    "position": {
                        "lat": 31.2304,
                        "lon": 121.4737
                    }
                }
            ]
        },
        "metadata": {
            "platform": "windows",
            "app_version": "0.1.0",
            "capabilities": {
                "os": "windows",
                "auto_upload": True
            }
        }
    }
    
    print("\n📤 发送数据:")
    print(json.dumps(payload, indent=2, ensure_ascii=False)[:1500] + "...")
    
    try:
        url = f"{WORKERS_BASE_URL}/api/node-info"
        print(f"\n🌐 POST {url}")
        
        response = requests.post(
            url,
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )
        
        print(f"\n📥 响应状态: {response.status_code}")
        print(f"📥 响应内容: {response.text}")
        
        if response.status_code == 200:
            data = response.json()
            if data.get("success"):
                print("\n✅ 完整节点信息上传成功！")
                return True
            else:
                print(f"\n❌ 上传失败: {data.get('message', 'Unknown error')}")
                return False
        else:
            print(f"\n❌ HTTP错误: {response.status_code}")
            return False
            
    except requests.exceptions.Timeout:
        print("\n❌ 请求超时")
        return False
    except requests.exceptions.ConnectionError:
        print("\n❌ 连接错误，请检查网络或后端服务状态")
        return False
    except Exception as e:
        print(f"\n❌ 请求失败: {e}")
        return False

def test_upload_without_iroh():
    """测试不上传iroh节点信息（节点未运行时）"""
    print("\n" + "="*60)
    print("📡 测试仅设备信息上传（无iroh节点）")
    print("="*60)
    
    device_id = f"test-device-{int(time.time())}"
    
    payload = {
        "device_id": device_id,
        "node_id": device_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "device_info": {
            "gpu_type": "NVIDIA GeForce GTX 1650 Ti",
            "gpu_usage": 0.0,
            "gpu_memory_total": 4.0,
            "gpu_memory_used": 0.0,
            "cpu_cores": 12,
            "total_memory_gb": 7.42,
            "battery_level": None,
            "is_charging": None
        },
        "iroh_node": None,  # 节点未运行
        "metadata": {
            "platform": "windows",
            "app_version": "0.1.0",
            "capabilities": {
                "os": "windows",
                "auto_upload": True,
                "node_running": False
            }
        }
    }
    
    print("\n📤 发送数据:")
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    
    try:
        url = f"{WORKERS_BASE_URL}/api/node-info"
        print(f"\n🌐 POST {url}")
        
        response = requests.post(
            url,
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )
        
        print(f"\n📥 响应状态: {response.status_code}")
        print(f"📥 响应内容: {response.text}")
        
        if response.status_code == 200:
            data = response.json()
            if data.get("success"):
                print("\n✅ 设备信息上传成功（无iroh节点）！")
                return True
            else:
                print(f"\n❌ 上传失败: {data.get('message', 'Unknown error')}")
                return False
        else:
            print(f"\n❌ HTTP错误: {response.status_code}")
            return False
            
    except Exception as e:
        print(f"\n❌ 请求失败: {e}")
        return False

if __name__ == "__main__":
    print("="*60)
    print("🚀 完整节点信息上传测试")
    print("="*60)
    print(f"后端地址: {WORKERS_BASE_URL}")
    
    results = {}
    
    # 测试1: 完整信息（含iroh）
    results["full_upload"] = test_full_node_upload()
    
    # 测试2: 仅设备信息
    results["device_only"] = test_upload_without_iroh()
    
    # 总结
    print("\n" + "="*60)
    print("📊 测试总结")
    print("="*60)
    
    for test_name, passed in results.items():
        status = "✅ 通过" if passed else "❌ 失败"
        print(f"   {test_name}: {status}")
    
    all_passed = all(results.values())
    print("\n" + "="*60)
    if all_passed:
        print("✅ 所有测试通过！自动上传功能正常工作")
    else:
        print("❌ 部分测试失败")
    print("="*60)
