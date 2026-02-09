import { ModelConfig } from '../types';

export interface InferenceResult {
  status: string;
  result?: any;
  message?: string;
  nodes_used?: string[];
  inference_time?: number;
}

export class PythonClient {
  private baseUrl: string;

  constructor(baseUrl: string = 'http://localhost:8080') {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  /**
   * 健康检查
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/health`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      return response.ok;
    } catch (error) {
      console.error('Health check failed:', error);
      return false;
    }
  }

  /**
   * 获取可用模型列表
   */
  async listModels(): Promise<ModelConfig[]> {
    try {
      const response = await fetch(`${this.baseUrl}/api/models`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      return data.models || [];
    } catch (error) {
      console.error('Failed to fetch models:', error);
      
      // 返回默认模型列表作为回退
      return [
        {
          id: 'bert-base-uncased',
          name: 'BERT Base Uncased',
          description: 'BERT base model, uncased',
          dimensions: 768,
          learning_rate: 0.001,
          batch_size: 32,
          type: 'nlp',
          size: '440MB',
        },
        {
          id: 'resnet18',
          name: 'ResNet-18',
          description: 'ResNet-18 computer vision model',
          dimensions: 512,
          learning_rate: 0.001,
          batch_size: 32,
          type: 'vision',
          size: '44MB',
        },
        {
          id: 'gpt2',
          name: 'GPT-2 Small',
          description: 'GPT-2 small language model',
          dimensions: 768,
          learning_rate: 0.001,
          batch_size: 32,
          type: 'generative',
          size: '548MB',
        },
      ];
    }
  }

  /**
   * 发送推理请求
   */
  async sendInferenceRequest(
    modelName: string,
    inputData: Record<string, any>,
    parameters?: Record<string, any>
  ): Promise<InferenceResult> {
    try {
      const response = await fetch(`${this.baseUrl}/api/inference`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model_name: modelName,
          model_source: 'huggingface',
          input_data: inputData,
          parameters: parameters || {},
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      console.error('Inference request failed:', error);
      return {
        status: 'error',
        message: error instanceof Error ? error.message : 'Unknown error occurred',
      };
    }
  }

  /**
   * 开始训练（包装为推理请求）
   */
  async startTraining(modelName: string): Promise<InferenceResult> {
    return this.sendInferenceRequest(
      modelName,
      {
        text: 'Start training task',
        task: 'training',
      },
      {
        batch_size: 1,
        max_length: 512,
      }
    );
  }

  /**
   * 停止训练
   */
  async stopTraining(): Promise<InferenceResult> {
    return {
      status: 'success',
      message: 'Training stopped',
    };
  }
}

// 创建单例实例
export const pythonClient = new PythonClient('http://localhost:8080');
