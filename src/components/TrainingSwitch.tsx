import React, { useState } from 'react';
import {
  Box,
  Switch,
  Card,
  CardContent,
  useTheme,
  alpha,
  Snackbar,
  Alert,
} from '@mui/material';
import { useTrainingStore } from '../store/trainingStore';
import { useModelStore } from '../store/modelStore';
import { pythonClient } from '../utils/pythonClient';

export const TrainingSwitch: React.FC = () => {
  const theme = useTheme();
  const { isRunning, setRunning } = useTrainingStore();
  const { selectedModel } = useModelStore();
  const [loading, setLoading] = useState(false);
  const [notification, setNotification] = useState<{ open: boolean; message: string; severity: 'success' | 'error' }>({
    open: false,
    message: '',
    severity: 'success',
  });

  const handleToggle = async () => {
    if (loading) return;

    setLoading(true);
    try {
      if (isRunning) {
        // 停止训练
        const result = await pythonClient.stopTraining();
        console.log('Training stopped:', result);
        setRunning(false);
        setNotification({
          open: true,
          message: '训练已停止',
          severity: 'success',
        });
      } else {
        // 开始训练前检查Python服务状态
        const isHealthy = await pythonClient.healthCheck();
        if (!isHealthy) {
          setNotification({
            open: true,
            message: '无法连接到边缘服务器，请确保Python服务已启动',
            severity: 'error',
          });
          setLoading(false);
          return;
        }

        // 检查是否选择了模型
        if (!selectedModel) {
          setNotification({
            open: true,
            message: '请先选择一个模型',
            severity: 'error',
          });
          setLoading(false);
          return;
        }

        // 发送训练请求到Python服务
        const result = await pythonClient.startTraining(selectedModel);
        console.log('Training started:', result);
        
        if (result.status === 'success') {
          setRunning(true);
          setNotification({
            open: true,
            message: `训练已启动，使用模型: ${selectedModel}`,
            severity: 'success',
          });
        } else {
          setNotification({
            open: true,
            message: `启动训练失败: ${result.message}`,
            severity: 'error',
          });
        }
      }
    } catch (error) {
      console.error('Error toggling training:', error);
      setNotification({
        open: true,
        message: '操作失败: ' + (error instanceof Error ? error.message : '未知错误'),
        severity: 'error',
      });
    } finally {
      setLoading(false);
    }
  };

  const handleCloseNotification = () => {
    setNotification({ ...notification, open: false });
  };

  return (
    <Box
      sx={{
        position: 'absolute',
        top: 16,
        left: 16,
        zIndex: 10,
      }}
    >
      <Card
        sx={{
          background: alpha(theme.palette.background.paper, 0.9),
          backdropFilter: 'blur(10px)',
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 1,
          width: 56,
          height: 56,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <CardContent sx={{ p: 1 }}>
          <Switch
            checked={isRunning}
            onChange={handleToggle}
            disabled={loading}
            size="medium"
            sx={{
              '& .MuiSwitch-switchBase.Mui-checked': {
                color: '#4caf50',
              },
              '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': {
                backgroundColor: '#4caf50',
              },
            }}
          />
        </CardContent>
      </Card>
      
      {/* 通知提示 */}
      <Snackbar
        open={notification.open}
        autoHideDuration={5000}
        onClose={handleCloseNotification}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert
          onClose={handleCloseNotification}
          severity={notification.severity}
          sx={{ width: '100%' }}
        >
          {notification.message}
        </Alert>
      </Snackbar>
    </Box>
  );
};
