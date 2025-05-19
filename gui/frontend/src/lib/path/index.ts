// src/lib/path/index.ts (or use alias '@/lib/path/index.ts')

const GLOB_REGEXP = /[*?[{\]]/;
const PATH_SEP_REGEXP = /[\\/]+$/;

/**
 * Extracts the base directory path from a glob pattern.
 * For example, given 'D:\\path\\to\\dir\\*', it returns 'D:\\path\\to\\dir'.
 * It removes the portion of the path starting from the first glob character (`*`, `?`, `[`, `{`).
 *
 * @param globPath - The file path possibly containing glob patterns
 * @returns The base path with glob characters removed
 */
export function stripGlob(globPath: string): string {
  // Matches common glob characters: *, ?, [, {
  const globCharIndex = globPath.search(GLOB_REGEXP);
  if (globCharIndex === -1) {
    return globPath;
  }

  // Slice up to the first glob character and remove trailing path separators
  const basePath = globPath.slice(0, globCharIndex);
  return basePath.replace(PATH_SEP_REGEXP, ''); // remove trailing slashes or backslashes
}
