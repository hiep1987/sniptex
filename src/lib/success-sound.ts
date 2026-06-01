let audioContext: AudioContext | null = null;

export async function playSuccessSound(enabled: boolean): Promise<void> {
  if (!enabled) return;
  try {
    const ctx = getAudioContext();
    if (ctx.state === "suspended") {
      await ctx.resume();
    }

    const now = ctx.currentTime;
    playTone(ctx, now, 880, 0.05, 0.035);
    playTone(ctx, now + 0.055, 1175, 0.08, 0.03);
  } catch (err) {
    console.warn("[sound] success sound failed", err);
  }
}

function getAudioContext(): AudioContext {
  if (audioContext) return audioContext;
  audioContext = new AudioContext();
  return audioContext;
}

function playTone(
  ctx: AudioContext,
  start: number,
  frequency: number,
  duration: number,
  gainValue: number,
) {
  const oscillator = ctx.createOscillator();
  const gain = ctx.createGain();

  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(frequency, start);
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(gainValue, start + 0.01);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);

  oscillator.connect(gain);
  gain.connect(ctx.destination);
  oscillator.start(start);
  oscillator.stop(start + duration + 0.02);
}
