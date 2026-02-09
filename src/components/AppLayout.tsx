import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Box } from '@mui/material';
import { TrainingSwitch } from './TrainingSwitch';
import { SettingsButton } from './SettingsButton';
import { SettingsPanel } from './SettingsPanel';
import { Resizer } from './Resizer';
import { LeftPanel } from './LeftPanel';
import { RightPanel } from './RightPanel';
import { useTrainingStore } from '../store/trainingStore';

export const AppLayout: React.FC = () => {
  const { isSettingsOpen, openSettings, closeSettings } = useTrainingStore();
  const [splitPercentage, setSplitPercentage] = useState(70); // 左侧初始占比70%
  const [isRightPanelVisible, setIsRightPanelVisible] = useState(true);
  const [isDragging, setIsDragging] = useState(false);
  const splitPercentageRef = useRef(splitPercentage);

  // 同步 splitPercentage 到 ref
  useEffect(() => {
    splitPercentageRef.current = splitPercentage;
  }, [splitPercentage]);

  const handleResize = useCallback((percentage: number) => {
    setSplitPercentage(percentage);
    splitPercentageRef.current = percentage;
  }, []);

  const handleMinimize = () => {
    setIsRightPanelVisible(false);
    setSplitPercentage(95); // 右侧隐藏时，左侧占95%
  };

  const handleShowPanel = () => {
    setIsRightPanelVisible(true);
    setSplitPercentage(70); // 显示时恢复到70%
  };

  const handleDragStart = useCallback(() => {
    setIsDragging(true);
  }, []);

  const handleDragEnd = useCallback(() => {
    setIsDragging(false);
    // 使用 ref 获取最新值，避免闭包问题
    const currentPercentage = splitPercentageRef.current;
    // 拖动结束后，检查是否需要自动隐藏/显示面板
    if (currentPercentage >= 92 && isRightPanelVisible) {
      // 隐藏右侧面板
      setIsRightPanelVisible(false);
      setSplitPercentage(95);
      splitPercentageRef.current = 95;
    } else if (currentPercentage < 95 && !isRightPanelVisible) {
      // 显示右侧面板
      setIsRightPanelVisible(true);
      setSplitPercentage(70);
      splitPercentageRef.current = 70;
    }
  }, [isRightPanelVisible]);

  return (
    <Box
      sx={{
        width: '100vw',
        height: '100vh',
        position: 'relative',
        background: 'black',
        display: 'flex',
      }}
    >
      {/* 左侧训练开关 */}
      <TrainingSwitch />

      {/* 右上角设置按钮 */}
      <SettingsButton onClick={openSettings} />

      {/* 主内容区域 - 水平布局 */}
      <Box
        sx={{
          flex: 1,
          height: '100%',
          display: 'flex',
          overflow: 'hidden',
          ml: '80px', // 为左侧训练开关留出空间
        }}
      >
        {/* 左侧：训练仪表盘 + 模型选择器 */}
        <LeftPanel splitPercentage={splitPercentage} />

        {/* 可拖拽的分界线 */}
        <Box
          onClick={!isRightPanelVisible ? handleShowPanel : undefined}
          sx={{
            flexShrink: 0, // 防止被压缩
            height: '100%',
            cursor: !isRightPanelVisible ? 'pointer' : 'ew-resize',
            position: 'relative',
            // 当右侧隐藏时，增大可点击区域
            '::after': !isRightPanelVisible ? {
              content: '""',
              position: 'absolute',
              right: 0,
              width: '40px',
              height: '100%',
            } : {},
          }}
        >
          <Resizer
            onResize={handleResize}
            onMinimize={handleMinimize}
            isRightPanelVisible={isRightPanelVisible}
            isDragging={isDragging}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
          />
        </Box>

        {/* 右侧：聊天框 */}
        <RightPanel
          isRightPanelVisible={isRightPanelVisible}
          splitPercentage={splitPercentage}
          onShowPanel={handleShowPanel}
        />
      </Box>

      {/* 设置面板 */}
      {isSettingsOpen && <SettingsPanel onClose={closeSettings} />}
    </Box>
  );
};
