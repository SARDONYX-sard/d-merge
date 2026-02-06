import { OutFormat } from '@/services/api/serde_hkx';
import { FileTab } from '../types/FileTab';

/** Global editor state */
export type EditorState = {
  tabs: FileTab[];
  active: number;
  showPreview: boolean;
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
  | { type: 'MARK_SAVED'; index: number; hkanno: FileTab['hkanno'] };
