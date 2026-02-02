#!/usr/bin/env python3
"""
测试节点信息收集和上传到Workers后端
"""

import json
import requests
import time
from datetime import datetime

# Workers后端地址
WORKERS_BASE_URL = "https://williw.sirazede725.workers.dev"

def get_gpu_info():
    """获取GPU信息 - 模拟Rust中的 williw::device::DeviceDetector::detect_gpu_usage()"""
    gpu_info = {
        "gpu_type": None,
        "gpu_usage": None,
        "gpu_memory_total": None,
        "gpu_memory_used": None,
    }
    
    try:
        # 尝试使用nvidia-smi获取GPU信息
        import subprocess
        result = subprocess.run(
            ["nvidia-smi", "--query-gpu=name,utilization.gpu,memory.total,memory.used", 
             "--format=csv,noheader,nounits"],
            capture_output=True, text=True, timeout=5
        )
        
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            if lines:
                parts = lines[0].split(',')
                if len(parts) >= 4:
                    gpu_info["gpu_type"] = parts[0].strip()
                    gpu_info["gpu_usage"] = float(parts[1].strip())
                    gpu_info["gpu_memory_total"] = float(parts[2].strip()) / 1024  # MB to GB
                    gpu_info["gpu_memory_used"] = float(parts[3].strip()) / 1024  # MB to GB
                    print(f"✅ GPU信息获取成功: {gpu_info}")
                else:
                    print(f"⚠️ nvidia-smi输出格式不正确: {lines[0]}")
        else:
            print(f"⚠️ nvidia-smi返回错误码: {result.returncode}")
    except FileNotFoundError:
        print("⚠️ nvidia-smi未找到，请确保NVIDIA驱动已安装")
    except Exception as e:
        print(f"⚠️ 获取GPU信息失败: {e}")
    
    return gpu_info

def get_system_info():
    """获取系统信息 - 模拟sysinfo"""
    import psutil
    
    cpu_cores = psutil.cpu_count(logical=True)
    total_memory_gb = psutil.virtual_memory().total / (1024**3)
    
    return {
        "cpu_cores": cpu_cores,
        "total_memory_gb": round(total_memory_gb, 2),
    }

def test_local_collection():
    """测试本地设备信息收集"""
    print("\n" + "="*60)
    print("📊 测试1: 本地设备信息收集")
    print("="*60)
    
    # 收集GPU信息
    gpu_info = get_gpu_info()
    print(f"\n🎮 GPU信息:")
    print(f"   GPU类型: {gpu_info.get('gpu_type') or 'N/A'}")
    print(f"   GPU使用率: {gpu_info.get('gpu_usage') or 'N/A'}%")
    print(f"   GPU显存总量: {gpu_info.get('gpu_memory_total') or 'N/A'} GB")
    print(f"   GPU显存使用: {gpu_info.get('gpu_memory_used') or 'N/A'} GB")
    
    # 收集系统信息
    sys_info = get_system_info()
    print(f"\n💻 系统信息:")
    print(f"   CPU核心数: {sys_info['cpu_cores']}")
    print(f"   总内存: {sys_info['total_memory_gb']} GB")
    
    # 合并信息
    device_info = {
        **gpu_info,
        **sys_info,
        "battery_level": None,
        "is_charging": None,
    }
    
    print("\n✅ 本地设备信息收集完成")
    return device_info

def test_upload_to_workers(device_info):
    """测试上传到Workers后端"""
    print("\n" + "="*60)
    print("📡 测试2: 上传到Workers后端")
    print("="*60)
    print(f"后端地址: {WORKERS_BASE_URL}")
    
    # 构建payload (模仿 DeviceInfoPayload)
    device_id = f"test-device-{int(time.time())}"
    payload = {
        "device_id": device_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "node_id": device_id,  # 后端要求必填
        "device_info": device_info,
        "metadata": {
            "platform": "windows",
            "app_version": "0.1.0",
            "capabilities": {
                "os": "windows",
                "test": True,
            }
        }
    }
    
    print("\n📤 发送数据:")
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    
    try:
        # 上传到 /api/node-info
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
            try:
                data = response.json()
                if data.get("success"):
                    print("\n✅ 设备信息上传成功！")
                    return True
                else:
                    print(f"\n❌ 上传失败: {data.get('message', 'Unknown error')}")
                    return False
            except:
                print("\n⚠️ 响应不是有效的JSON")
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

def test_health_check():
    """测试Workers后端健康检查"""
    print("\n" + "="*60)
    print("🏥 测试3: Workers后端健康检查")
    print("="*60)
    
    try:
        url = f"{WORKERS_BASE_URL}/api/health"
        print(f"🌐 GET {url}")
        
        response = requests.get(url, timeout=10)
        print(f"📥 响应状态: {response.status_code}")
        
        if response.status_code == 200:
            print(f"📥 响应内容: {response.text}")
            print("\n✅ Workers后端健康检查通过")
            return True
        else:
            print(f"\n⚠️ 健康检查返回非200状态码: {response.status_code}")
            return False
            
    except Exception as e:
        print(f"\n❌ 健康检查失败: {e}")
        return False

def test_get_node_info():
    """测试从Workers获取节点信息"""
    print("\n" + "="*60)
    print("📋 测试4: 从Workers获取节点信息")
    print("="*60)
    
    try:
        url = f"{WORKERS_BASE_URL}/api/nodes"
        print(f"🌐 GET {url}")
        
        response = requests.get(url, timeout=10)
        print(f"📥 响应状态: {response.status_code}")
        print(f"📥 响应内容: {response.text[:500]}...")
        
        if response.status_code == 200:
            print("\n✅ 成功获取节点列表")
            return True
        else:
            print(f"\n⚠️ 获取节点列表失败: {response.status_code}")
            return False
            
    except Exception as e:
        print(f"\n❌ 获取节点信息失败: {e}")
        return False

if __name__ == "__main__":
    print("="*60)
    print("🚀 节点信息上传测试工具")
    print("="*60)
    
    results = {}
    
    # 测试1: 健康检查
    results["health_check"] = test_health_check()
    
    # 测试2: 本地收集
    device_info = test_local_collection()
    
    # 测试3: 上传到Workers
    results["upload"] = test_upload_to_workers(device_info)
    
    # 测试4: 获取节点信息
    results["get_nodes"] = test_get_node_info()
    
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
        print("✅ 所有测试通过！GPU节点信息可以成功传递给Workers后端")
    else:
        print("❌ 部分测试失败，请检查以上错误信息")
    print("="*60)
