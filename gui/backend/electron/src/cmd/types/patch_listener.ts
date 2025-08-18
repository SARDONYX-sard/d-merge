type StatusIndexing = {
  /** 1 based index */
  index: number;
  total: number;
};
/** Error message from the backend */
type ErrorPayload = string;

/**
 * Backend status enum for merge operation (defined in Rust).
 *
 * The backend emits these status values using `window.emit(...)` during various stages.
 */
export type Status =
  | { type: 'ReadingPatches'; content: StatusIndexing }
  | { type: 'ParsingPatches'; content: StatusIndexing }
  | { type: 'ApplyingPatches'; content: StatusIndexing }
  | { type: 'GeneratingHkxFiles'; content: StatusIndexing }
  | { type: 'Done' }
  | { type: 'Error'; content: ErrorPayload };
