import { create } from 'zustand';

interface UIState {
  // 右侧面板显示状态
  isRightPanelVisible: boolean;
  
  // 设置右侧面板显示状态
  setRightPanelVisible: (visible: boolean) => void;
  
  // 显示右侧面板（展开对话框）
  showRightPanel: () => void;
  
  // 隐藏右侧面板
  hideRightPanel: () => void;
  
  // 切换右侧面板
  toggleRightPanel: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  isRightPanelVisible: true,
  
  setRightPanelVisible: (visible) => set({ isRightPanelVisible: visible }),
  
  showRightPanel: () => set({ isRightPanelVisible: true }),
  
  hideRightPanel: () => set({ isRightPanelVisible: false }),
  
  toggleRightPanel: () => set((state) => ({ isRightPanelVisible: !state.isRightPanelVisible })),
}));
