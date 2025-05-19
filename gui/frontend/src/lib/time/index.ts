export function elapsedToText(startMs: number): string {
  const endMs = performance.now();
  const elapsed = endMs - startMs;
  const seconds = Math.floor(elapsed / 1000);
  const ms = Math.floor(elapsed % 1000);
  return `${seconds}.${ms.toString().padStart(3, '0')}s`;
}
