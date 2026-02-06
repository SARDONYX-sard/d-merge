'use client';

import { useRouter, useRouterState } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { z } from 'zod';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { schemaStorage } from '@/lib/storage/schemaStorage';

/**
 * usePageRedirect (TanStack Router)
 *
 * Handles:
 * - One-time redirect to lastPath
 * - Keeping lastPath up to date when user navigates
 * - Returning current selected index for UI
 */
export const usePageRedirect = <T extends Readonly<[string, ...string[]]>>(validPaths: T) => {
  const router = useRouter();

  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const pathSchema = z.enum(validPaths);

  const [lastPath, setLastPath] = schemaStorage.use(PUB_CACHE_OBJ.lastPath, pathSchema);

  const [selectedIndex, setSelectedIndex] = useState(0);

  const normalizePath = (path: string): (typeof validPaths)[number] => {
    for (const name of validPaths) {
      if (name === '/') continue;
      if (path.endsWith(name) || path.endsWith(`${name}/`)) {
        return name;
      }
    }
    return '/';
  };

  const currentPath = normalizePath(pathname);

  // --- Redirect once per session ---
  useEffect(() => {
    if (!lastPath) return;

    const hasRedirected = sessionStorage.getItem('hasRedirected');
    if (hasRedirected) return;
    if (lastPath === '/' || pathname.endsWith(lastPath)) return;

    sessionStorage.setItem('hasRedirected', 'true');

    router.navigate({
      to: lastPath,
      replace: true,
    });
  }, [lastPath, pathname, router]);

  // --- Sync lastPath & selectedIndex ---
  useEffect(() => {
    const index = validPaths.indexOf(currentPath);
    setSelectedIndex(index >= 0 ? index : 0);
    setLastPath(currentPath);
  }, [currentPath, validPaths, setLastPath]);

  const navigateTo = (index: number) => {
    const target = validPaths[index];
    if (!target) return;

    setSelectedIndex(index);

    router.navigate({
      to: target,
    });
  };

  return {
    selectedIndex,
    navigateTo,
  };
};
