import React, { useState, useEffect, useRef } from 'react';
import {
  Box,
  Switch,
  Card,
  CardContent,
  Typography,
  useTheme,
  alpha,
  Snackbar,
  Alert,
} from '@mui/material';
import { useTrainingStore } from '../store/trainingStore';
import { useUIStore } from '../store/uiStore';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';

export const TrainingSwitch: React.FC = () => {
  const theme = useTheme();
  const { isRunning, setRunning } = useTrainingStore();
  const { showRightPanel } = useUIStore();
  const [loading, setLoading] = useState(false);
  const [notification, setNotification] = useState<{ open: boolean; message: string; severity: 'success' | 'error' | 'info' | 'warning' }>({
    open: false,
    message: '',
    severity: 'success',
  });
  
  const pollIntervalRef = useRef<number | null>(null);
  const lastPollTimeRef = useRef<string>('');

  useEffect(() => {
    console.log('🔧 [TrainingSwitch] Component mounted, isRunning:', isRunning);
    
    if (isRunning) {
      startPolling();
    }
    
    return () => {
      stopPolling();
    };
  }, []);

  useEffect(() => {
    if (isRunning) {
      startPolling();
    } else {
      stopPolling();
    }
  }, [isRunning]);

  const startPolling = () => {
    if (pollIntervalRef.current) return;
    
    console.log('📡 [TrainingSwitch] 开始轮询 Workers 消息...');
    
    pollIntervalRef.current = window.setInterval(async () => {
      try {
        const result = await invoke<{ success: boolean; messages: any[]; poll_timestamp: string }>('poll_workers_messages', {
          lastPollTime: lastPollTimeRef.current || undefined,
        });
        
        if (result.success && result.messages.length > 0) {
          console.log('📨 [TrainingSwitch] 收到新消息:', result.messages.length);
          
          // 更新轮询时间
          lastPollTimeRef.current = result.poll_timestamp;
          
          // 处理每条消息
          for (const msg of result.messages) {
            await handleWorkersMessage(msg);
          }
        }
      } catch (error) {
        console.log('📡 [TrainingSwitch] 轮询失败:', error);
      }
    }, 10000); // 每 10 秒轮询一次
  };

  const stopPolling = () => {
    if (pollIntervalRef.current) {
      console.log('📡 [TrainingSwitch] 停止轮询...');
      window.clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  };

  const handleWorkersMessage = async (msg: any) => {
    console.log('📨 [TrainingSwitch] 处理消息:', msg.message_type);
    
    switch (msg.message_type) {
      case 'node_connection_request':
        // AI 处理节点连接请求
        await handleAiNodeConnection(msg);
        break;
        
      case 'training_task':
        // 收到训练任务
        showRightPanel();
        await emit('workflow-message', {
          type: 'info',
          content: `📬 收到训练任务\n\n来自节点: ${msg.from_node}\n内容: ${JSON.stringify(msg.content)}`,
        });
        break;
        
      case 'model_shard':
        // 收到模型分片
        showRightPanel();
        await emit('workflow-message', {
          type: 'info',
          content: `📦 收到模型分片\n\n来自: ${msg.from_node}`,
        });
        break;
        
      default:
        console.log('📨 [TrainingSwitch] 未知消息类型:', msg.message_type);
    }
  };

  const handleAiNodeConnection = async (msg: any) => {
    try {
      showRightPanel();
      
      await emit('workflow-message', {
        type: 'info',
        content: `🤖 AI 正在分析节点连接请求...\n\n来自: ${msg.from_node}`,
      });
      
      const result = await invoke<any>('handle_ai_node_connection', {
        connectionRequest: {
          from_node: msg.from_node,
          from_node_info: msg.content?.node_info,
          suggested_connection: msg.content?.suggested_connection,
        },
      });
      
      if (result.decision === 'accepted') {
        await emit('workflow-message', {
          type: 'success',
          content: `✅ AI 决定接受连接\n\n🔗 ${result.from_node}\n💡 ${result.ai_reasoning}`,
        });
      } else {
        await emit('workflow-message', {
          type: 'warning',
          content: `⏳ AI 建议延迟连接\n\n💡 ${result.ai_reasoning}`,
        });
      }
    } catch (error) {
      console.error('❌ [TrainingSwitch] AI 处理连接失败:', error);
    }
  };

  const handleToggle = async () => {
    console.log('🎯 [TrainingSwitch] handleToggle, isRunning:', isRunning);
    
    if (loading) return;
    setLoading(true);
    
    try {
      if (isRunning) {
        // 停止 - 停止轮询
        console.log('📤 [TrainingSwitch] 停止...');
        stopPolling();
        setRunning(false);
        setNotification({
          open: true,
          message: '已停止，已断开 Workers 连接',
          severity: 'success',
        });
      } else {
        // 启动 - 传送本地信息给 workers
        console.log('📤 [TrainingSwitch] 启动...');
        showRightPanel();
        
        await emit('workflow-message', {
          type: 'info',
          content: '📡 正在向 Workers 广播本地信息...\n\n将传输：\n• 设备信息\n• GPU 状态\n• 网络配置\n\n同时开始轮询消息...',
        });

        const result = await invoke<{ success: boolean; message: string }>('upload_full_node_info_to_workers', {});
        console.log('✅ [TrainingSwitch] 传输成功:', result);
        
        await emit('workflow-message', {
          type: 'success',
          content: `✅ ${result?.message || '本地信息已成功传送到 Workers 网络'}\n\n🔄 开始轮询消息...`,
        });
        
        setRunning(true);
        setNotification({
          open: true,
          message: '✅ 已连接，正在轮询...',
          severity: 'success',
        });
      }
    } catch (error) {
      console.error('❌ [TrainingSwitch] 失败:', error);
      setNotification({
        open: true,
        message: '操作失败: ' + (error instanceof Error ? error.message : '未知错误'),
        severity: 'error',
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ position: 'absolute', top: 16, left: 16, zIndex: 10 }}>
      <Card sx={{
        background: alpha(theme.palette.background.paper, 0.9),
        backdropFilter: 'blur(10px)',
        border: `1px solid ${theme.palette.divider}`,
        borderRadius: 1,
        width: 56,
        height: 56,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}>
        <CardContent sx={{ p: 1 }}>
          <Switch
            checked={isRunning}
            onChange={handleToggle}
            disabled={loading}
            size="medium"
            sx={{
              '& .MuiSwitch-switchBase.Mui-checked': { color: '#4caf50' },
              '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': { backgroundColor: '#4caf50' },
            }}
          />
        </CardContent>
      </Card>
      
      <Snackbar open={notification.open} autoHideDuration={5000} onClose={() => setNotification({ ...notification, open: false })}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}>
        <Alert severity={notification.severity} sx={{ width: '100%' }}>
          {notification.message}
        </Alert>
      </Snackbar>
    </Box>
  );
};
