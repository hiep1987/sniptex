import { useState } from "react";
import { tauri } from "@/lib/invoke";
import { cn } from "@/lib/cn";
import { strings } from "@/strings";

export default function OnboardingWindow() {
  const [step, setStep] = useState(0);
  const total = strings.onboarding.steps.length;
  const isLast = step === total - 1;

  return (
    <main className="flex h-dvh w-dvw flex-col bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <header className="border-b border-slate-200 px-6 py-4 dark:border-slate-800">
        <h1 className="text-base font-semibold tracking-tight">
          {strings.onboarding.title}
        </h1>
        <ol className="mt-3 flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
          {strings.onboarding.steps.map((label, idx) => (
            <li
              key={label}
              className={cn(
                "flex items-center gap-1",
                idx === step && "text-slate-900 dark:text-slate-100",
              )}
            >
              <span
                className={cn(
                  "flex size-5 items-center justify-center rounded-full text-[10px] font-semibold",
                  idx <= step
                    ? "bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
                    : "bg-slate-200 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
                )}
              >
                {idx + 1}
              </span>
              <span>{label}</span>
              {idx < total - 1 && <span className="text-slate-300">›</span>}
            </li>
          ))}
        </ol>
      </header>

      <section className="flex-1 overflow-auto px-6 py-8">
        <OnboardingStepBody step={step} />
      </section>

      <footer className="flex items-center justify-between border-t border-slate-200 px-6 py-3 dark:border-slate-800">
        <button
          type="button"
          onClick={() => tauri.hideWindow("onboarding")}
          className="text-xs text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
        >
          {strings.onboarding.skip}
        </button>
        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={step === 0}
            onClick={() => setStep((s) => Math.max(0, s - 1))}
            className="rounded-md border border-slate-300 px-3 py-1.5 text-xs disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-700"
          >
            {strings.onboarding.back}
          </button>
          <button
            type="button"
            onClick={() => {
              if (isLast) tauri.hideWindow("onboarding");
              else setStep((s) => Math.min(total - 1, s + 1));
            }}
            className="rounded-md bg-slate-900 px-3 py-1.5 text-xs font-medium text-white hover:bg-slate-800 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white"
          >
            {isLast ? strings.onboarding.finish : strings.onboarding.next}
          </button>
        </div>
      </footer>
    </main>
  );
}

function OnboardingStepBody({ step }: { step: number }) {
  const label = strings.onboarding.steps[step] ?? "";
  return (
    <div className="max-w-lg">
      <h2 className="mb-2 text-lg font-semibold">{label}</h2>
      <p className="text-sm text-slate-500 dark:text-slate-400">
        Phase 8 will fill this step with real onboarding content (agent
        install guides, API-key entry, hotkey picker, first-run summary).
      </p>
    </div>
  );
}
