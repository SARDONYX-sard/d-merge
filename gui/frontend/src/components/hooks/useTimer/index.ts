import { useRef, useState } from 'react';

export const useTimer = () => {
  const [elapsed, setElapsed] = useState(0);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  const start = () => {
    setElapsed(0);
    timerRef.current = setInterval(() => {
      setElapsed((prev) => prev + 100);
    }, 100);
  };

  const stop = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
  };

  const seconds = Math.floor(elapsed / 1000);
  const ms = Math.floor(elapsed % 1000);
  const text = `${seconds}.${ms.toString().padStart(3, '0')}s`;

  return {
    elapsed,
    seconds,
    ms,
    text,
    start,
    stop,
  };
};
