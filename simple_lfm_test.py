#!/usr/bin/env python3
"""
简单的LFM模型测试
"""
import torch
from transformers import AutoTokenizer, AutoModelForCausalLM

print("🚀 LFM2.5-1.2B-Thinking 简单测试")

# 检查GPU
print(f"CUDA可用: {torch.cuda.is_available()}")
if torch.cuda.is_available():
    print(f"GPU: {torch.cuda.get_device_name(0)}")

# 模型路径
model_path = r"D:\AI\去中心化训练\test_models\models--LiquidAI--LFM2.5-1.2B-Thinking\snapshots\1c9725ba97f047b37bcf53e44e9133ccf1f79333"

try:
    print("🔄 加载tokenizer...")
    tokenizer = AutoTokenizer.from_pretrained(model_path)
    print("✅ Tokenizer加载成功")
    
    print("🧠 加载模型...")
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    
    # 使用更小的模型参数
    model = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
        low_cpu_mem_usage=True,
        trust_remote_code=True
    )
    
    if torch.cuda.is_available():
        model = model.to(device)
    
    print("✅ 模型加载成功")
    print(f"✅ 模型设备: {next(model.parameters()).device}")
    
    # 简单测试
    print("\n🧪 推理测试...")
    prompt = "你好"
    inputs = tokenizer(prompt, return_tensors="pt")
    inputs = {key: val.to(device) for key, val in inputs.items()}
    
    with torch.no_grad():
        outputs = model.generate(
            **inputs,
            max_new_tokens=20,
            do_sample=False,  # 使用确定性生成
            pad_token_id=tokenizer.eos_token_id
        )
    
    result = tokenizer.decode(outputs[0], skip_special_tokens=True)
    if result.startswith(prompt):
        result = result[len(prompt):].strip()
    
    print(f"📝 输入: {prompt}")
    print(f"🤖 输出: {result}")
    print("🎉 测试成功!")
    
except Exception as e:
    print(f"❌ 错误: {e}")
    import traceback
    traceback.print_exc()
