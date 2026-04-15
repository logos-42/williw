import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface WalletStore {
  walletAddress: string;
  isConnected: boolean;
  setWalletAddress: (address: string) => void;
  clearWallet: () => void;
}

export const useWalletStore = create<WalletStore>()(
  persist(
    (set) => ({
      walletAddress: '',
      isConnected: false,
      setWalletAddress: (address: string) => set({ walletAddress: address, isConnected: !!address }),
      clearWallet: () => set({ walletAddress: '', isConnected: false }),
    }),
    {
      name: 'wallet-storage',
    }
  )
);