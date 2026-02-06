import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { schemaStorage } from '@/lib/storage/schemaStorage';
import { hkannoToText } from '@/services/api/hkanno';
import type { OutFormat } from '@/services/api/serde_hkx';
import { FileTab } from '../types/FileTab';
import type { EditorAction, EditorState } from './editorTypes';

/** Reducer handling editor state transitions */
export const editorReducer = (state: EditorState, action: EditorAction): EditorState => {
  switch (action.type) {
    case 'OPEN_TABS': {
      const existingIds = new Set(state.tabs.map((t) => t.id));

      const newTabs: FileTab[] = [];
      const skippedIds: string[] = [];

      for (const tab of action.tabs) {
        if (existingIds.has(tab.id)) {
          skippedIds.push(tab.id);
          continue;
        }
        newTabs.push(tab);
      }

      if (skippedIds.length > 0) {
        console.info('[Editor] skipped tabs due to duplicate id:', skippedIds);
      }

      if (newTabs.length === 0) {
        return state;
      }

      const tabs = [...state.tabs, ...newTabs];

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);

      return {
        ...state,
        tabs,
        active: state.tabs.length,
      };
    }

    case 'SET_ACTIVE':
      schemaStorage.set<number>(PRIVATE_CACHE_OBJ.hkannoActiveTab, action.index);

      return { ...state, active: action.index };

    case 'CLOSE_TAB': {
      const tabs = state.tabs.filter((_, i) => i !== action.index);
      const active = Math.max(0, Math.min(state.active, tabs.length - 1));

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);
      schemaStorage.set<number>(PRIVATE_CACHE_OBJ.hkannoActiveTab, action.index);

      return { ...state, tabs, active };
    }

    case 'REVERT_ACTIVE_TAB': {
      const tabs = [...state.tabs];

      const activeTab = tabs[state.active];
      tabs[state.active] = {
        ...activeTab,
        text: hkannoToText(activeTab.hkanno),
        dirty: false,
      };
      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);

      return { ...state, tabs };
    }

    case 'UPDATE_TEXT': {
      const tabs = [...state.tabs];
      tabs[state.active] = { ...tabs[state.active], text: action.text, dirty: true };

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);

      return { ...state, tabs };
    }

    case 'UPDATE_CURSOR': {
      const tabs = [...state.tabs];
      tabs[state.active] = { ...tabs[state.active], cursorPos: action.cursorPos };

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);
      return { ...state, tabs };
    }

    case 'UPDATE_OUTPUT': {
      const tabs = [...state.tabs];
      tabs[state.active] = { ...tabs[state.active], outputPath: action.outputPath };

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);
      return { ...state, tabs };
    }

    case 'UPDATE_FORMAT': {
      const tabs = [...state.tabs];
      const activateTab = tabs[state.active];
      tabs[state.active] = {
        ...activateTab,
        format: action.format,
        outputPath: changeExtension(activateTab.outputPath, action.format),
      };

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);
      return { ...state, tabs };
    }

    case 'TOGGLE_PREVIEW':
      schemaStorage.set<boolean>(PRIVATE_CACHE_OBJ.hkannoShowPreview, !state.showPreview);
      return { ...state, showPreview: !state.showPreview };

    /** From rust */
    case 'MARK_SAVED': {
      const tabs = [...state.tabs];
      tabs[action.index] = { ...tabs[action.index], dirty: false, hkanno: action.hkanno };

      schemaStorage.set<FileTab[]>(PRIVATE_CACHE_OBJ.hkannoFileTabs, tabs);
      return { ...state, tabs };
    }

    default:
      return state;
  }
};

const changeExtension = (outputPath: string, format: OutFormat): string => {
  const idx = outputPath.lastIndexOf('.');
  const base = idx === -1 ? outputPath : outputPath.slice(0, idx);
  switch (format) {
    case 'amd64':
    case 'win32':
      return `${base}.hkx`;
    case 'xml':
      return `${base}.xml`;
    case 'json':
      return `${base}.json`;
    default:
      return outputPath;
  }
};
