#!/usr/bin/env python3
"""
GPU 推理服务器
提供本地 GPU 推理服务，支持模型加载和推理

API 端点:
- GET  /         - 健康检查
- POST /infer    - 执行推理
- POST /load_model - 加载模型
- GET  /models   - 获取已加载的模型列表
- DELETE /models/<model_id> - 卸载模型
"""

import os
import sys
import json
import time
import uuid
import argparse
from datetime import datetime
from typing import Optional, Dict, List
from functools import wraps

from flask import Flask, request, jsonify
from flask_cors import CORS

# 配置
app = Flask(__name__)
CORS(app)

# 全局状态
class ServerState:
    def __init__(self):
        self.loaded_models: Dict[str, dict] = {}
        self.device = "cpu"
        self.torch_available = False
        self.transformers_available = False
        self.server_start_time = time.time()
        
    def check_dependencies(self):
        """检查依赖是否可用"""
        try:
            import torch
            self.torch_available = True
            self.device = "cuda" if torch.cuda.is_available() else "cpu"
            print(f"✅ PyTorch 可用，使用设备: {self.device}")
            if self.device == "cuda":
                print(f"   GPU: {torch.cuda.get_device_name(0)}")
                print(f"   显存: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")
        except ImportError:
            print("⚠️ PyTorch 未安装")
            
        try:
            import transformers
            self.transformers_available = True
            print(f"✅ Transformers 可用 (版本: {transformers.__version__})")
        except ImportError:
            print("⚠️ Transformers 未安装")

state = ServerState()

# 错误处理
def handle_errors(f):
    @wraps(f)
    def decorated(*args, **kwargs):
        try:
            return f(*args, **kwargs)
        except Exception as e:
            print(f"❌ 错误: {str(e)}")
            import traceback
            traceback.print_exc()
            return jsonify({
                "status": "error",
                "message": str(e),
                "error_type": type(e).__name__
            }), 500
    return decorated

# ============ API 端点 ============

@app.route("/", methods=["GET"])
@handle_errors
def health_check():
    """健康检查端点"""
    uptime = time.time() - state.server_start_time
    return jsonify({
        "status": "healthy",
        "service": "williw-gpu-inference-server",
        "version": "1.0.0",
        "uptime_seconds": int(uptime),
        "device": state.device,
        "torch_available": state.torch_available,
        "transformers_available": state.transformers_available,
        "loaded_models": len(state.loaded_models),
        "timestamp": datetime.now().isoformat()
    })

@app.route("/infer", methods=["POST"])
@handle_errors
def run_inference():
    """执行推理"""
    data = request.get_json()
    
    if not data:
        return jsonify({
            "status": "error",
            "message": "请求体不能为空"
        }), 400
    
    # 获取参数
    model_path = data.get("model_path", "")
    input_text = data.get("input_text", "")
    max_length = data.get("max_length", 512)
    
    if not input_text:
        return jsonify({
            "status": "error", 
            "message": "input_text 不能为空"
        }), 400
    
    # 生成请求 ID
    request_id = str(uuid.uuid4())[:8]
    print(f"📝 [{request_id}] 推理请求: {input_text[:50]}...")
    
    start_time = time.time()
    
    # 检查是否有模型加载
    if not state.transformers_available:
        # 如果没有 transformers，返回模拟响应
        processing_time = time.time() - start_time
        return jsonify({
            "status": "success",
            "message": "推理完成（模拟模式 - Transformers 未安装）",
            "request_id": request_id,
            "result": f"这是一个模拟的推理结果。输入: {input_text[:100]}",
            "processing_time": round(processing_time, 2),
            "mode": "mock"
        })
    
    # 尝试使用真实模型推理
    try:
        result = perform_real_inference(model_path, input_text, max_length)
        processing_time = time.time() - start_time
        
        return jsonify({
            "status": "success",
            "message": "推理完成",
            "request_id": request_id,
            "result": result,
            "processing_time": round(processing_time, 2),
            "device": state.device,
            "mode": "real"
        })
        
    except Exception as e:
        # 如果真实推理失败，返回模拟响应
        processing_time = time.time() - start_time
        print(f"⚠️ 真实推理失败，使用模拟: {e}")
        
        return jsonify({
            "status": "success",
            "message": "推理完成（模拟模式 - 真实推理失败）",
            "request_id": request_id,
            "result": f"模拟结果: 我理解你在说 '{input_text[:50]}...'。这是一个基于规则的响应，因为模型推理遇到问题。",
            "processing_time": round(processing_time, 2),
            "mode": "mock",
            "error": str(e)
        })

def perform_real_inference(model_path: str, input_text: str, max_length: int) -> str:
    """执行真实推理"""
    from transformers import AutoModelForCausalLM, AutoTokenizer
    import torch
    
    # 如果没有指定模型，使用默认模型
    if not model_path:
        model_path = "microsoft/DialoGPT-medium"  # 默认使用小模型
    
    # 检查模型是否已加载
    if model_path not in state.loaded_models:
        print(f"📥 加载模型: {model_path}")
        tokenizer = AutoTokenizer.from_pretrained(model_path)
        model = AutoModelForCausalLM.from_pretrained(model_path)
        model = model.to(state.device)
        
        state.loaded_models[model_path] = {
            "model": model,
            "tokenizer": tokenizer,
            "loaded_at": datetime.now().isoformat()
        }
    
    model_info = state.loaded_models[model_path]
    tokenizer = model_info["tokenizer"]
    model = model_info["model"]
    
    # 编码输入
    input_ids = tokenizer.encode(input_text + tokenizer.eos_token, return_tensors="pt")
    input_ids = input_ids.to(state.device)
    
    # 生成响应
    with torch.no_grad():
        output = model.generate(
            input_ids,
            max_length=max_length,
            pad_token_id=tokenizer.eos_token_id,
            do_sample=True,
            temperature=0.7,
        )
    
    # 解码响应
    response = tokenizer.decode(output[:, input_ids.shape[-1]:][0], skip_special_tokens=True)
    
    return response.strip()

@app.route("/load_model", methods=["POST"])
@handle_errors
def load_model():
    """加载模型"""
    data = request.get_json()
    
    if not data or "model_path" not in data:
        return jsonify({
            "status": "error",
            "message": "需要提供 model_path"
        }), 400
    
    model_path = data["model_path"]
    
    if not state.transformers_available:
        return jsonify({
            "status": "error",
            "message": "Transformers 未安装，无法加载模型"
        }), 500
    
    try:
        from transformers import AutoModelForCausalLM, AutoTokenizer
        
        print(f"📥 加载模型: {model_path}")
        tokenizer = AutoTokenizer.from_pretrained(model_path)
        model = AutoModelForCausalLM.from_pretrained(model_path)
        
        if state.device == "cuda":
            model = model.to("cuda")
        
        model_id = str(uuid.uuid4())[:8]
        state.loaded_models[model_id] = {
            "model_path": model_path,
            "model": model,
            "tokenizer": tokenizer,
            "loaded_at": datetime.now().isoformat()
        }
        
        return jsonify({
            "status": "success",
            "message": f"模型加载成功",
            "model_id": model_id,
            "model_path": model_path,
            "device": state.device
        })
        
    except Exception as e:
        return jsonify({
            "status": "error",
            "message": f"模型加载失败: {str(e)}"
        }), 500

@app.route("/models", methods=["GET"])
@handle_errors
def get_models():
    """获取已加载的模型列表"""
    models = []
    for model_id, info in state.loaded_models.items():
        models.append({
            "model_id": model_id,
            "path": info.get("model_path", "unknown"),
            "loaded_at": info.get("loaded_at", ""),
            "status": "loaded"
        })
    
    return jsonify({
        "loaded_models": len(models),
        "models": models,
        "device": state.device
    })

@app.route("/models/<model_id>", methods=["DELETE"])
@handle_errors
def unload_model(model_id: str):
    """卸载模型"""
    if model_id not in state.loaded_models:
        return jsonify({
            "status": "error",
            "message": f"模型 '{model_id}' 不存在"
        }), 404
    
    # 删除模型释放内存
    del state.loaded_models[model_id]
    
    # 强制垃圾回收
    import gc
    gc.collect()
    
    if state.device == "cuda":
        import torch
        torch.cuda.empty_cache()
    
    return jsonify({
        "status": "success",
        "message": f"模型 '{model_id}' 已卸载"
    })

@app.route("/status", methods=["GET"])
@handle_errors
def get_status():
    """获取详细状态"""
    import psutil
    
    # 系统信息
    memory = psutil.virtual_memory()
    
    status = {
        "server": {
            "uptime_seconds": int(time.time() - state.server_start_time),
            "device": state.device,
            "torch_available": state.torch_available,
            "transformers_available": state.transformers_available,
        },
        "system": {
            "cpu_percent": psutil.cpu_percent(),
            "memory_used_gb": round(memory.used / 1024**3, 2),
            "memory_total_gb": round(memory.total / 1024**3, 2),
            "memory_percent": memory.percent,
        },
        "models": {
            "loaded_count": len(state.loaded_models),
            "models": [
                {"id": k, "path": v.get("model_path", "unknown")}
                for k, v in state.loaded_models.items()
            ]
        }
    }
    
    # GPU 信息
    if state.device == "cuda":
        import torch
        status["gpu"] = {
            "name": torch.cuda.get_device_name(0),
            "memory_allocated_gb": round(torch.cuda.memory_allocated() / 1024**3, 2),
            "memory_reserved_gb": round(torch.cuda.memory_reserved() / 1024**3, 2),
        }
    
    return jsonify(status)

# ============ 主程序 ============

def main():
    parser = argparse.ArgumentParser(description="Williw GPU 推理服务器")
    parser.add_argument("--port", type=int, default=8000, help="服务器端口")
    parser.add_argument("--host", type=str, default="0.0.0.0", help="服务器地址")
    parser.add_argument("--debug", action="store_true", help="启用调试模式")
    
    args = parser.parse_args()
    
    print("=" * 60)
    print("🚀 Williw GPU 推理服务器")
    print("=" * 60)
    
    # 检查依赖
    state.check_dependencies()
    
    print("-" * 60)
    print(f"📡 服务地址: http://{args.host}:{args.port}")
    print("📋 API 端点:")
    print(f"   - GET  http://localhost:{args.port}/")
    print(f"   - POST http://localhost:{args.port}/infer")
    print(f"   - POST http://localhost:{args.port}/load_model")
    print(f"   - GET  http://localhost:{args.port}/models")
    print("-" * 60)
    print("按 Ctrl+C 停止服务器")
    print("=" * 60)
    
    # 启动服务器
    app.run(
        host=args.host,
        port=args.port,
        debug=args.debug,
        threaded=True
    )

if __name__ == "__main__":
    main()
