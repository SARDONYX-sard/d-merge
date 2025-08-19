/** NOTE: Do not use yaml because it cannot be reversed. */
export type OutputFormat = 'amd64' | 'win32' | 'xml' | 'json';

export type TreeViewBaseItem = {
  id: string;
  label: string;
  children?: TreeViewBaseItem[];
};
