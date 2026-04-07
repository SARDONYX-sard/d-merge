import { useRef, useState } from 'react';

export const useTimer = () => {
  const startTimeRef = useRef<number | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  const start = () => {
    startTimeRef.current = performance.now();
    timerRef.current = setInterval(() => {
      if (startTimeRef.current != null) {
        const now = performance.now();
        setElapsed(now - startTimeRef.current);
      }
    }, 300);
  };

  const stop = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    if (startTimeRef.current != null) {
      const now = performance.now();
      const total = now - startTimeRef.current;
      startTimeRef.current = null;

      const seconds = Math.floor(total / 1000);
      const ms = Math.floor(total % 1000);
      return `${seconds}.${ms.toString().padStart(3, '0')}s`;
    }

    return '0.000s';
  };

  const seconds = Math.floor(elapsed / 1000);
  const ms = Math.floor(elapsed % 1000);
  const text = `${seconds}.${ms.toString().padStart(3, '0')}s`;

  return {
    elapsed,
    seconds,
    text,
    ms,
    start,
    stop,
  };
};
