import { create } from 'zustand';

interface InferenceResult {
  request_id?: string;
  selected_nodes?: any[];
  estimated_total_time?: number;
  result?: string;
}

export interface NodeSplit {
  shortId: string;
  fullId: string;
  vram: number;
  layers: [number, number];
  isLocal: boolean;
  gpuType?: string;
}

export interface ActiveInferenceSession {
  /** Display name of the model, e.g. "Qwen2.5-72B" */
  modelName: string;
  /** HuggingFace repo, e.g. "Qwen/Qwen2.5-72B-Instruct" */
  modelRepo: string;
  /** param count string, e.g. "72B" */
  params: string;
  /** total layers */
  totalLayers: number;
  /** Pipeline split plan */
  splitPlan: NodeSplit[];
  /** true = single node / local only, false = multi-node distributed */
  isLocalOnly: boolean;
  /** When the session was activated */
  activatedAt: Date;
  /**
   * Local inference endpoint discovered by the AI agent, e.g. "http://localhost:11434/v1"
   * When set, ChatBox routes messages here instead of the external API.
   */
  inferenceEndpoint?: string;
  /** Model name to use on the local endpoint, e.g. "qwen2.5:7b" */
  localModelName?: string;
}

interface ModelStore {
  selectedModel: string | null;
  inferenceResult: InferenceResult | null;
  isInferenceLoading: boolean;
  /** The currently active inference session (set after clicking Run) */
  activeSession: ActiveInferenceSession | null;

  setSelectedModel: (model: string) => void;
  clearSelectedModel: () => void;
  setInferenceResult: (result: InferenceResult | null) => void;
  setInferenceLoading: (loading: boolean) => void;
  setActiveSession: (session: ActiveInferenceSession | null) => void;
  clearActiveSession: () => void;
}

export const useModelStore = create<ModelStore>((set) => ({
  selectedModel: null,
  inferenceResult: null,
  isInferenceLoading: false,
  activeSession: null,

  setSelectedModel: (model: string) => set({ selectedModel: model }),
  clearSelectedModel: () => set({ selectedModel: null }),
  setInferenceResult: (result: InferenceResult | null) => set({ inferenceResult: result }),
  setInferenceLoading: (loading: boolean) => set({ isInferenceLoading: loading }),
  setActiveSession: (session: ActiveInferenceSession | null) => set({ activeSession: session }),
  clearActiveSession: () => set({ activeSession: null }),
}));
