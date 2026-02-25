import { useEffect, useTransition } from 'react';
import { useDebounce } from '@/components/hooks/useDebounce';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { loadModsInfo } from '@/services/api/patch';
import { mergeModInfoList } from '@/services/api/patch/mod_item';

/**
 * Custom hook that handles fetching and updating mod info for the Patch page.
 *
 * ## Why this hook exists
 * Originally, `loadModsInfo()` was executed **inside the `PatchProvider`**.
 * That caused **unnecessary API requests** whenever the provider was mounted,
 * even on unrelated pages (e.g., MyPage, Settings, etc.) — because the provider
 * is global and re-used across routes.
 *
 * To avoid redundant fetches, the fetching logic was **moved out of the Provider**
 * and into this hook. This allows the data to be fetched **only on the Patch page**
 * (or any page that explicitly calls this hook).
 *
 * ## Responsibilities
 * - Debounces Skyrim data directory input changes to avoid spamming requests.
 * - Calls `loadModsInfo()` and converts the fetched result into `ModInfo[]`.
 * - Updates the context state (`setModInfoList`) provided by `PatchProvider`.
 * - Returns a loading state managed by React's `useTransition`.
 *
 * ## Usage
 * ```tsx
 * const { loading } = useFetchModInfo();
 * const { modInfoList } = usePatchContext();
 *
 * return (
 *   <>
 *     {loading && <Spinner />}
 *     <ModInfoList data={modInfoList} />
 *   </>
 * );
 * ```
 */
export const useFetchModInfo = () => {
  const { isVfsMode, vfsSkyrimDataDir, skyrimDataDir, setModList, setVfsModList } = usePatchContext();

  // Prevent excessive API calls while typing/changing directories.
  const [loading, startTransition] = useTransition();
  const deferredDir = useDebounce(isVfsMode ? vfsSkyrimDataDir : skyrimDataDir, 450).trim();

  useEffect(() => {
    if (!deferredDir) return;

    startTransition(() => {
      NOTIFY.asyncTry(async () => {
        const fetched = await loadModsInfo(deferredDir, isVfsMode);

        if (fetched.length > 0) {
          if (isVfsMode) {
            setVfsModList((prev) => mergeModInfoList(prev, fetched));
          } else {
            setModList((prev) => mergeModInfoList(prev, fetched));
          }
        }
      });
    });
  }, [deferredDir, isVfsMode]);

  return { loading };
};
