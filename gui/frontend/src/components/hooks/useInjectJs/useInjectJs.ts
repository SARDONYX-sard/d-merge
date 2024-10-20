import { useEffect, useRef } from 'react';

import { useJsContext } from '@/components/providers/JsProvider';
import { STORAGE } from '@/lib/storage';

/**
 * Inject JavaScript
 * By calling this hook on a page-by-page basis, js is executed at each page transition.
 *
 * # Note
 * If we load it with `layout.tsx`, it doesn't apply for some reason.
 */
export function useInjectJs() {
  const { js, setJs } = useJsContext();
  const script = useRef<HTMLScriptElement | null>(null);
  // # HACK: To avoid double call `useEffect`
  // If there is no cleanup function (during development), double mounting will not occur.
  //
  // However, since we want to perform cleanup and pass the test, we set a separate flag and
  // do not cleanup the ref of the flag to achieve the purpose.
  const isMounted = useRef(false);

  useEffect(() => {
    const runScript = STORAGE.getHidden('run-script');
    if (!(runScript && runScript === 'true') || isMounted.current || script.current) {
      return; // Skip if already run
    }
    isMounted.current = true;

    const scriptElement = document.createElement('script');
    scriptElement.innerHTML = js;
    document.body.appendChild(scriptElement); // Throw `DOMException`
    script.current = scriptElement;

    return () => {
      script.current?.remove();
      script.current = null;
    };
  }, [js]);

  return { js, setJs } as const;
}
