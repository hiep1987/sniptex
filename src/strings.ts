// Centralized UI strings. v1 ships English-only; future i18n bundles
// can swap this module without touching call sites.
export const strings = {
  app: {
    name: "SnipTeX",
    tagline: "Free OCR snip tool for LaTeX and Markdown.",
  },
  preview: {
    copy: "Copy",
    copied: "Copied",
    copyAs: "Copy as…",
    pin: "Pin",
    pinned: "Pinned",
    unpin: "Unpin",
    dismiss: "Dismiss",
    emptyTitle: "Waiting for snip",
    emptyHint: "Press Cmd/Ctrl + Shift + M to capture a region.",
    renderError: "Failed to render output",
  },
  copyAs: {
    raw: "Raw OCR text",
    tex: "TeX",
    plain: "Plain text",
    markdown: "Markdown",
    mathml: "MathML",
  },
  settings: {
    title: "Settings",
    tabs: {
      general: "General",
      agents: "Agents",
      hotkeys: "Hotkeys",
      formats: "Formats",
      about: "About",
    },
  },
  history: {
    title: "History",
    searchPlaceholder: "Search snips…",
    empty: "No snips yet. Capture one with the hotkey to populate history.",
  },
  onboarding: {
    title: "Welcome to SnipTeX",
    next: "Next",
    back: "Back",
    finish: "Finish",
    skip: "Skip",
    steps: [
      "Welcome",
      "Install agent",
      "Cloud key",
      "Hotkey",
      "Ready",
    ],
  },
} as const;
