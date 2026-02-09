import React from 'react';
import { Box } from '@mui/material';
import { TrainingDashboard } from './TrainingDashboard';
import { ModelSelector } from './ModelSelector';

interface LeftPanelProps {
  splitPercentage: number;
}

export const LeftPanel: React.FC<LeftPanelProps> = ({ splitPercentage }) => {
  return (
    <Box
      sx={{
        flex: `0 0 ${splitPercentage}%`,
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        minWidth: '30%', // 左侧最小30%，不能被最小化
        maxWidth: '95%', // 左侧最大95%，允许右侧隐藏
      }}
    >
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
        <TrainingDashboard />
      </Box>

      {/* 模型选择器 - 固定在左下角 */}
      <Box sx={{ flexShrink: 0, p: 2, background: 'rgba(0, 0, 0, 0.3)' }}>
        <ModelSelector />
      </Box>
    </Box>
  );
};
