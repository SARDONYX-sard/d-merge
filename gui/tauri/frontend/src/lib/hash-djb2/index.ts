/**
 * # Why use this?
 * The frontend selection can be deleted.
 * Therefore, the conversion status shifts when using index.
 * So, using hash from path solves this problem.
 * The exact same hash function is implemented in backend and tested.
 */
export function hashDjb2(key: string): number {
  let hash = 5381;
  for (let i = 0; i < key.length; i++) {
    hash = ((hash << 5) + hash) ^ key.charCodeAt(i);
  }
  return hash >>> 0; // Cast to u32
}
