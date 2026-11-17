export interface WorkerAllocation {
  resolvedStyles: number;
  chromium: number;
  total: number;
}

export const RESOLVED_STYLES_SHARE = 0.5;
export const CHROMIUM_WORKERS = 1;

export function allocateWorkers(logicalCpus: number): WorkerAllocation {
  const resolvedStyles = Math.max(
    1,
    Math.floor(logicalCpus * RESOLVED_STYLES_SHARE),
  );
  return {
    resolvedStyles,
    chromium: CHROMIUM_WORKERS,
    total: resolvedStyles + CHROMIUM_WORKERS,
  };
}
