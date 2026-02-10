import { create } from 'zustand';

interface WorkflowMessage {
  type: 'info' | 'progress' | 'success' | 'error' | 'warning';
  content: string;
  step?: string;
  progress?: number;
  timestamp: Date;
}

interface WorkflowStatus {
  isRunning: boolean;
  isCompleted: boolean;
  isFirstRun: boolean;
  currentStep: string;
  progress: number;
  message: string;
}

interface WorkflowStore {
  status: WorkflowStatus;
  messages: WorkflowMessage[];
  isFirstTime: boolean;

  setStatus: (status: WorkflowStatus) => void;
  addMessage: (message: WorkflowMessage) => void;
  clearMessages: () => void;
  setFirstTime: (firstTime: boolean) => void;
  reset: () => void;
}

export const useWorkflowStore = create<WorkflowStore>((set) => ({
  status: {
    isRunning: false,
    isCompleted: false,
    isFirstRun: true,
    currentStep: '',
    progress: 0,
    message: '',
  },
  messages: [],
  isFirstTime: true,

  setStatus: (status) => set((state) => ({
    status: { ...state.status, ...status },
  })),

  addMessage: (message) => set((state) => ({
    messages: [...state.messages, { ...message, timestamp: new Date() }],
  })),

  clearMessages: () => set({ messages: [] }),

  setFirstTime: (firstTime) => set({ isFirstTime: firstTime }),

  reset: () => set({
    status: {
      isRunning: false,
      isCompleted: false,
      isFirstRun: true,
      currentStep: '',
      progress: 0,
      message: '',
    },
    messages: [],
    isFirstTime: false,
  }),
}));
