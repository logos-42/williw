import { create } from 'zustand';

interface InferenceResult {
  request_id?: string;
  selected_nodes?: any[];
  estimated_total_time?: number;
  result?: string;
}

interface ModelStore {
  selectedModel: string | null;
  inferenceResult: InferenceResult | null;
  isInferenceLoading: boolean;
  
  setSelectedModel: (model: string) => void;
  clearSelectedModel: () => void;
  setInferenceResult: (result: InferenceResult | null) => void;
  setInferenceLoading: (loading: boolean) => void;
}

export const useModelStore = create<ModelStore>((set) => ({
  selectedModel: null,
  inferenceResult: null,
  isInferenceLoading: false,
  
  setSelectedModel: (model: string) => set({ selectedModel: model }),
  clearSelectedModel: () => set({ selectedModel: null }),
  setInferenceResult: (result: InferenceResult | null) => set({ inferenceResult: result }),
  setInferenceLoading: (loading: boolean) => set({ isInferenceLoading: loading }),
}));
