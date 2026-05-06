import type { HkAnnoLspOptions } from '../../MonacoEditor/support_hkanno';
import type { FileTab } from '../types/FileTab';
import type { OutFormat } from '@/services/api/serde_hkx';

/** Global editor state */
export type EditorState = {
  tabs: FileTab[];
  active: number;
  showPreview: boolean;
  lspOptions: HkAnnoLspOptions;
};

/** Editor reducer actions */
export type EditorAction =
  | { type: 'OPEN_TABS'; tabs: FileTab[] }
  | { type: 'SET_ACTIVE'; index: number }
  | { type: 'CLOSE_TAB'; index: number }
  | { type: 'REVERT_ACTIVE_TAB' }
  | { type: 'UPDATE_TEXT'; text: string }
  | { type: 'UPDATE_CURSOR'; cursorPos: FileTab['cursorPos'] }
  | { type: 'UPDATE_OUTPUT'; outputPath: string }
  | { type: 'UPDATE_FORMAT'; format: OutFormat }
  | { type: 'TOGGLE_PREVIEW' }
  | { type: 'MARK_SAVED'; index: number; hkanno: FileTab['hkanno'] }
  | { type: 'SET_LSP_OPTIONS'; lspOptions: HkAnnoLspOptions };
