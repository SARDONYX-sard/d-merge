/** NOTE: Do not use yaml because it cannot be reversed. */
export type OutputFormat = 'amd64' | 'win32' | 'xml' | 'json';

/** Payload for progress reporting */
export interface Payload {
  /**
   * Hashed identifier of the file path.
   *
   * Using a hash ensures that the frontend can track tasks reliably,
   * even if items are removed or reordered.
   *
   * - conversion input path to `djb2` hashed -> id
   */
  pathId: number;
  /** Current progress status of this task. */
  status: number;
}
