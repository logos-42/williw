import React from 'react';
import { Box } from '@mui/material';
import { ChatBox } from './ChatBox';

interface RightPanelProps {
  isRightPanelVisible: boolean;
  splitPercentage: number;
  onShowPanel?: () => void;
}

export const RightPanel: React.FC<RightPanelProps> = ({
  isRightPanelVisible,
  onShowPanel
}) => {
  return (
    <>
      {/* 右侧悬浮触发区域（当面板隐藏时） - 扩大点击区域 */}
      {!isRightPanelVisible && (
        <Box
          onClick={onShowPanel}
          sx={{
            flex: 0,
            width: 60, // 增大到60px
            height: '100%',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            '&:hover': {
              background: 'rgba(76, 175, 80, 0.2)',
            },
          }}
        >
          <Box
            sx={{
              color: 'rgba(255, 255, 255, 0.5)',
              fontSize: '20px',
              fontWeight: 'bold',
              writingMode: 'vertical-rl',
              textOrientation: 'mixed',
            }}
          >
            聊天
          </Box>
        </Box>
      )}

      {/* 右侧：聊天框 */}
      <Box
        sx={{
          flex: 1, // 自动填充剩余空间，避免闪烁
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: 2,
          p: 2,
          minWidth: isRightPanelVisible ? 80 : 0,
          background: 'rgba(0, 0, 0, 0.3)',
          opacity: isRightPanelVisible ? 1 : 0,
        }}
      >
        {/* 聊天框 */}
        <Box sx={{ flex: 1, minHeight: 0 }}>
          <ChatBox expanded={false} onExpand={() => {}} />
        </Box>
      </Box>
    </>
  );
};
