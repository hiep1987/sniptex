import { Camera } from "lucide-react";

export default function WelcomeStep() {
  return (
    <div className="max-w-lg space-y-4">
      <div className="flex items-center gap-3">
        <Camera className="size-8 text-slate-900 dark:text-slate-100" />
        <h2 className="text-xl font-semibold">Welcome to SnipTeX</h2>
      </div>
      <p className="text-sm leading-relaxed text-slate-600 dark:text-slate-300">
        SnipTeX captures a region of your screen, runs OCR through your
        chosen agent, and copies clean <strong>LaTeX</strong> or{" "}
        <strong>Markdown</strong> to your clipboard — in seconds.
      </p>
      <ul className="space-y-2 text-sm text-slate-600 dark:text-slate-300">
        <li className="flex items-start gap-2">
          <span className="mt-0.5 text-green-600">✓</span>
          Free and open-source — no subscription needed
        </li>
        <li className="flex items-start gap-2">
          <span className="mt-0.5 text-green-600">✓</span>
          Privacy-first: bring your own agent or API key
        </li>
        <li className="flex items-start gap-2">
          <span className="mt-0.5 text-green-600">✓</span>
          Works with equations, tables, and mixed content
        </li>
      </ul>
      <p className="text-xs text-slate-400 dark:text-slate-500">
        Let's get you set up in under a minute.
      </p>
    </div>
  );
}
