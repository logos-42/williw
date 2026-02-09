import React, { useRef, useEffect } from 'react';
import { Box } from '@mui/material';

interface ResizerProps {
  onResize: (width: number) => void;
  onMinimize?: () => void;
  isRightPanelVisible?: boolean;
  isDragging?: boolean;
  onDragStart?: () => void;
  onDragEnd?: () => void;
}

export const Resizer: React.FC<ResizerProps> = ({
  onResize,
  onMinimize,
  isRightPanelVisible = true,
  isDragging = false,
  onDragStart,
  onDragEnd
}) => {
  const resizerRef = useRef<HTMLDivElement>(null);
  const onResizeRef = useRef(onResize);
  const rafIdRef = useRef<number>();
  const lastUpdateRef = useRef<number>(0);

  // 更新 ref 以保持最新的 onResize
  useEffect(() => {
    onResizeRef.current = onResize;
  }, [onResize]);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!resizerRef.current) return;

      // 获取主内容区域（Resizer的父容器的父容器）
      const mainContent = resizerRef.current.parentElement?.parentElement;
      if (!mainContent) return;

      const rect = mainContent.getBoundingClientRect();
      const newLeftWidth = e.clientX - rect.left;
      const percentage = (newLeftWidth / rect.width) * 100;

      // 限制在30%-95%之间，左侧最小30%，右侧可以完全隐藏
      const clampedPercentage = Math.max(30, Math.min(95, percentage));

      // 使用 requestAnimationFrame 节流更新，限制为 16ms（约60fps）
      const now = performance.now();
      if (now - lastUpdateRef.current < 16) {
        if (rafIdRef.current) {
          cancelAnimationFrame(rafIdRef.current);
        }
        rafIdRef.current = requestAnimationFrame(() => {
          onResizeRef.current(clampedPercentage);
          lastUpdateRef.current = now;
        });
      } else {
        onResizeRef.current(clampedPercentage);
        lastUpdateRef.current = now;
      }
    };

    const handleMouseUp = () => {
      document.body.style.cursor = 'default';
      // 清理 RAF
      if (rafIdRef.current) {
        cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = undefined;
      }
      // 调用拖动结束回调
      onDragEnd?.();
    };

    document.addEventListener('mousemove', handleMouseMove, { passive: true });
    document.addEventListener('mouseup', handleMouseUp);
    document.body.style.cursor = 'ew-resize';

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      if (rafIdRef.current) {
        cancelAnimationFrame(rafIdRef.current);
      }
    };
  }, [isDragging, onDragEnd]);

  return (
    <Box
      ref={resizerRef}
      onMouseDown={() => {
        onDragStart?.();
      }}
      onDoubleClick={onMinimize}
      sx={{
        width: isDragging ? 8 : 4,
        height: '100%',
        cursor: 'ew-resize',
        backgroundColor: isDragging ? 'rgba(76, 175, 80, 0.5)' : 'rgba(255, 255, 255, 0.1)',
        transition: isDragging ? 'none' : 'background-color 0.2s',
        '&:hover': {
          backgroundColor: 'rgba(76, 175, 80, 0.3)',
          width: 6,
        },
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        '&::before': {
          content: isRightPanelVisible ? '""' : '"<"',
          position: 'absolute',
          left: '50%',
          top: '50%',
          transform: 'translate(-50%, -50%)',
          width: isRightPanelVisible ? 2 : 'auto',
          height: isRightPanelVisible ? 40 : 'auto',
          backgroundColor: isRightPanelVisible ? 'rgba(255, 255, 255, 0.3)' : 'transparent',
          borderRadius: 1,
          color: 'rgba(255, 255, 255, 0.5)',
          fontSize: '16px',
          fontWeight: 'bold',
        },
      }}
    />
  );
};
