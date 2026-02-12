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
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { useUIStore } from '../store/uiStore';

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
  const { showRightPanel } = useUIStore();

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
    // 使用默认模型
    const modelId = selectedModel || 'lfm-2.5-1.2b-thinking';
    
    console.log('🚀 [ModelSelector] 启动AI配置工作流...模型:', modelId);
    
    // 展开右侧对话框
    showRightPanel();
    
    // 发送启动消息
    try {
      await emit('workflow-message', {
        type: 'info',
        content: '🎉 欢迎使用 Williw！\n\n🤖 AI 助手正在为您配置系统...\n\n📋 AI将自动完成：\n• 检测系统环境\n• 安装必要依赖\n• 配置 Iroh P2P 网络\n• 初始化去中心化节点\n\n⏳ 请稍候，这个过程大约需要几分钟...',
      });
    } catch (e) {
      console.warn('emit 失败，继续执行:', e);
    }

    setInferenceLoading(true);
    setInferenceError('');
    setInferenceResult(null);

    try {
      console.log('📤 [ModelSelector] 调用 start_document_driven_workflow...');
      // 启动AI配置工作流 (注意：Rust命令使用驼峰命名)
      const result = await invoke<string>('start_document_driven_workflow', { 
        apiKey: '', 
        modelPath: modelId 
      });
      console.log('✅ [ModelSelector] AI工作流已启动:', result);
      
      try {
        await emit('workflow-message', {
          type: 'success',
          content: '✅ AI配置工作流已启动！\n\n请查看右侧对话框查看配置进度...',
        });
      } catch (e) {
        console.warn('emit 成功消息失败:', e);
      }
      
    } catch (error) {
      console.error('❌ [ModelSelector] 启动工作流失败:', error);
      setInferenceError('启动工作流失败: ' + (error instanceof Error ? error.message : String(error)));
      
      // 同时显示到对话框
      try {
        await emit('workflow-message', {
          type: 'error',
          content: '❌ 启动失败: ' + (error instanceof Error ? error.message : String(error)),
        });
      } catch (e) {
        console.warn('emit 错误消息失败:', e);
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
              <Box sx={{ flex: 1, display: 'flex', alignItems: 'center' }}>
                <Typography variant="caption" sx={{ mr: 1, color: 'text.secondary', whiteSpace: 'nowrap' }}>
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
              
              <Box sx={{ display: 'flex', alignItems: 'center', height: '100%' }}>
                <Button
                  variant="contained"
                  startIcon={isInferenceLoading ? <CircularProgress size={16} /> : <PlayArrowIcon />}
                  onClick={handleInferenceRequest}
                  disabled={isInferenceLoading}
                  sx={{
                    px: 2,
                    py: 0.75,
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
