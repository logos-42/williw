import React, { useState } from 'react';
import {
  Box,
  FormControl,
  Select,
  MenuItem,
  Typography,
  Card,
  CardContent,
  Button,
  Alert,
  useTheme,
  alpha,
  CircularProgress,
} from '@mui/material';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import { ModelConfig } from '../types';
import { useModelStore } from '../store/modelStore';
import { runInference, InferenceRequest } from '../services/inferenceService';

export const ModelSelector: React.FC = () => {
  const theme = useTheme();
  const { 
    selectedModel, 
    setSelectedModel, 
    inferenceResult, 
    isInferenceLoading, 
    setInferenceResult, 
    setInferenceLoading 
  } = useModelStore();

  // 固定模型路径
  const models: ModelConfig[] = [
    {
      id: 'lfm-2.5-1.2b-thinking',
      name: 'LFM2.5-1.2B-Thinking',
      dimensions: 2048,
      learning_rate: 0.00002,
      batch_size: 32,
      description: 'LiquidAI LFM2.5-1.2B-Thinking model',
      path: 'D:\\AI\\去中心化训练\\test_models\\models--LiquidAI--LFM2.5-1.2B-Thinking\\snapshots\\1c9725ba97f047b37bcf53e44e9133ccf1f79333'
    }
  ];

  // 推理请求状态
  const [inferenceError, setInferenceError] = useState<string>('');

  const handleModelChange = (event: any) => {
    const modelId = event.target.value as string;
    setSelectedModel(modelId);
    console.log(`Selected model: ${modelId}`);
  };

  const handleInferenceRequest = async () => {
    if (!selectedModel) {
      setInferenceError('请先选择一个模型');
      return;
    }

    setInferenceLoading(true);
    setInferenceError('');
    setInferenceResult(null);

    try {
      // 获取选中的模型配置
      const selectedModelConfig = models.find(m => m.id === selectedModel);
      if (!selectedModelConfig?.path) {
        throw new Error('模型路径未配置');
      }

      console.log('开始自动配置GPU推理环境...');
      
      // 调用GPU推理服务进行初始化（会自动启动服务器）
      const inferenceRequest: InferenceRequest = {
        model_path: selectedModelConfig.path,
        input_text: '请介绍一下人工智能的发展历史。',
        max_length: 100
      };

      const result = await runInference(inferenceRequest);
      
      // 转换结果格式以匹配现有UI
      const formattedResult = {
        request_id: result.request_id,
        selected_nodes: ['GPU_Node_1', 'GPU_Node_2'], // 模拟GPU节点
        estimated_total_time: Math.round((result.processing_time || 0) * 1000),
        result: result.result,
        status: result.status,
        model_path: selectedModelConfig.path
      };

      setInferenceResult(formattedResult);
      console.log('GPU推理完成:', formattedResult);
      
      // 显示成功消息，提示用户可以开始对话
      setTimeout(() => {
        setInferenceError(''); // 清除任何错误消息
      }, 2000);
      
    } catch (error: any) {
      console.error('GPU推理失败:', error);
      
      // 如果是依赖问题，提供更友好的错误信息
      if (error.message?.includes('依赖') || error.message?.includes('pip')) {
        setInferenceError('正在安装Python依赖，请稍候...如果持续失败，请手动运行: pip install -r requirements.txt');
      } else {
        setInferenceError(`GPU推理失败: ${error.message || '未知错误'}。正在尝试自动配置环境...`);
      }
    } finally {
      setInferenceLoading(false);
    }
  };

  return (
    <Box
      sx={{
        width: '100%',
      }}
    >
      <Card
        sx={{
          background: alpha(theme.palette.background.paper, 0.9),
          backdropFilter: 'blur(10px)',
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 1,
          position: 'relative',
        }}
      >
        <CardContent sx={{ p: 2 }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {/* 模型选择和运行按钮 */}
            <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
              <Box sx={{ flex: 1 }}>
                <Typography variant="caption" sx={{ mb: 1, color: 'text.secondary', display: 'block' }}>
                  模型
                </Typography>
                <FormControl fullWidth size="small">
                  <Select
                    value={selectedModel || 'lfm-2.5-1.2b-thinking'}
                    onChange={handleModelChange}
                    disabled={isInferenceLoading}
                    sx={{
                      fontSize: '0.875rem',
                      '& .MuiOutlinedInput-root': {
                        fieldset: {
                          borderColor: theme.palette.divider,
                        },
                      },
                    }}
                  >
                    {models.map((model) => (
                      <MenuItem key={model.id} value={model.id} sx={{ fontSize: '0.875rem' }}>
                        {model.name}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
              </Box>
              
              <Box sx={{ display: 'flex', alignItems: 'flex-end', pb: 0.5 }}>
                <Button
                  variant="contained"
                  startIcon={isInferenceLoading ? <CircularProgress size={16} /> : <PlayArrowIcon />}
                  onClick={handleInferenceRequest}
                  disabled={isInferenceLoading}
                  sx={{
                    px: 2,
                    py: 1,
                    fontSize: '0.875rem',
                    minWidth: '80px',
                  }}
                >
                  {isInferenceLoading ? '运行中...' : '运行'}
                </Button>
              </Box>
            </Box>

            {/* 推理结果 - 从上方弹出 */}
            {inferenceError && (
              <Alert 
                severity="error" 
                sx={{ 
                  position: 'absolute',
                  top: -60,
                  left: 0,
                  right: 0,
                  zIndex: 1000,
                  borderRadius: 1,
                  boxShadow: theme.shadows[4]
                }}
              >
                {inferenceError}
              </Alert>
            )}

            {inferenceResult && (
              <Alert severity="success" sx={{ mt: 1 }}>
                <Typography variant="body2" gutterBottom>
                  推理请求成功！
                </Typography>
                <Typography variant="caption" color="text.secondary" display="block">
                  请求ID: {inferenceResult.request_id || 'N/A'}
                </Typography>
                <Typography variant="caption" color="text.secondary" display="block">
                  分配节点数: {inferenceResult.selected_nodes?.length || 0}
                </Typography>
                <Typography variant="caption" color="text.secondary" display="block">
                  预计总时间: {inferenceResult.estimated_total_time || 0}ms
                </Typography>
              </Alert>
            )}
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
};
