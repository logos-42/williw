import { create } from 'zustand';
import { TrainingStatus } from '../types';

interface TrainingStore {
  isRunning: boolean;
  isSettingsOpen: boolean;
  trainingStatus: TrainingStatus | null;
  
  toggleTraining: () => void;
  setRunning: (running: boolean) => void;
  openSettings: () => void;
  closeSettings: () => void;
  setTrainingStatus: (status: TrainingStatus | null) => void;
}

export const useTrainingStore = create<TrainingStore>((set) => ({
  isRunning: true,
  isSettingsOpen: false,
  trainingStatus: null,
  
  toggleTraining: () => set((state) => ({ isRunning: !state.isRunning })),
  setRunning: (running) => set({ isRunning: running }),
  openSettings: () => set({ isSettingsOpen: true }),
  closeSettings: () => set({ isSettingsOpen: false }),
  setTrainingStatus: (status) => set({ trainingStatus: status }),
}));
