import { useEffect, useTransition } from 'react';
import { useDebounce } from '@/components/hooks/useDebounce';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { type FetchedModInfo, loadModsInfo, ModInfo } from '@/services/api/patch';

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
  const { isVfsMode, vfsSkyrimDataDir, skyrimDataDir, setFetchedModInfoList } = usePatchContext();
  const [loading, startTransition] = useTransition();

  // Prevent excessive API calls while typing/changing directories.
  const deferredDir = useDebounce(isVfsMode ? vfsSkyrimDataDir : skyrimDataDir, 450).trim();

  useEffect(() => {
    if (!deferredDir) return;

    startTransition(() => {
      NOTIFY.asyncTry(async () => {
        const fetched = await loadModsInfo(deferredDir);
        if (fetched.length > 0) {
          setFetchedModInfoList((prev) => mergeModInfoList(prev, fetched));
        }
      });
    });
  }, [deferredDir, isVfsMode]);

  return { loading };
};

/**
 * Merge fetched list into previous ModInfo[] by `id`.
 * - Preserve `enabled` and `priority` from prev when id matches.
 * - Keep latest name/author/site/auto/modType from fetched.
 * - If priority missing in prev, assign index+1 (fetched order).
 */
const mergeModInfoList = (prev: ModInfo[], fetched: FetchedModInfo[]): ModInfo[] => {
  const prevMap = new Map<string, ModInfo>();
  for (const p of prev) {
    prevMap.set(p.id, p);
  }

  let newIndex = fetched.length;

  const result = fetched.map((f) => {
    const existing = prevMap.get(f.id);

    const merged: ModInfo = {
      id: f.id,
      name: f.name,
      author: f.author,
      site: f.site,
      auto: f.auto,
      mod_type: f.mod_type,
      enabled: existing?.enabled ?? false,
      priority: existing?.priority ?? newIndex,
    };
    if (existing === undefined) {
      newIndex += 1;
    }

    return merged;
  });

  console.log('sort');
  result.sort((a, b) => a.priority - b.priority);

  return result;
};
