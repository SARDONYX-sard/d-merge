import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useEffect, useState } from 'react';

export const useTauriDragDrop = (openFiles: (paths: string[]) => Promise<void>) => {
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    (async () => {
      const webview = await getCurrentWebview();
      unlisten = await webview.onDragDropEvent(async (event) => {
        switch (event.payload.type) {
          case 'over':
            setDragging(true);
            break;
          case 'drop':
            setDragging(false);
            const paths = event.payload.paths;
            if (paths.length) {
              openFiles(paths);
            }
            break;
          case 'leave':
            setDragging(false);
          default:
            break;
        }
      });
    })();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return {
    dragging,
    setDragging,
  };
};
