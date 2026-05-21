import { create } from "zustand";

type HotkeyState = {
  pressCount: number;
  lastPressedAt: number | null;
  recordPress: () => void;
};

export const useHotkeyStore = create<HotkeyState>((set) => ({
  pressCount: 0,
  lastPressedAt: null,
  recordPress: () =>
    set((s) => ({
      pressCount: s.pressCount + 1,
      lastPressedAt: Date.now(),
    })),
}));
