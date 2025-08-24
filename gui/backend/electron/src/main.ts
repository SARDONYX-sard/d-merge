// SPDX-FileCopyrightText: (C) 2023 DarkGuy10
// SPDX-License-Identifier: Apache-2.0 OR MIT

import type { IpcMainInvokeEvent } from 'electron';
import { app, BrowserWindow, ipcMain, session } from 'electron';
import { type ContextMenuParams, DEFAULT_MENU, TEXT_MENU } from './context_menu';
import { handleAccessRequest } from './url_resolver';
import { createWindow } from './window';
import './cmd'; // load ipc handlers
import { loggerInit, logWarn } from 'd_merge_node';
import { LOG_DIR, LOG_FILE_NAME } from './cmd/log';

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// App event handlers

app.on('ready', () => {
  loggerInit(LOG_DIR, LOG_FILE_NAME);

  createWindow();
  if (app.isPackaged) {
    session.defaultSession.webRequest.onBeforeRequest(handleAccessRequest);
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// backend functions for frontend: Need `preload.ts` settings

// Toggle the menu to be displayed when right-clicking.
ipcMain.handle('show-context-menu', (_event: IpcMainInvokeEvent, params: ContextMenuParams) => {
  const win = BrowserWindow.getFocusedWindow();
  if (!win) {
    logWarn('No focused window to show context menu');
    return;
  }

  // With text selection → for copy/paste
  if (params.selectionText && params.selectionText.trim()! == '') {
    TEXT_MENU.popup({ window: win, x: params.x, y: params.y });
  } else {
    DEFAULT_MENU.popup({ window: win, x: params.x, y: params.y });
  }
});

ipcMain.handle('zoom', (_event, { delta }: { delta: number }) => {
  const win = BrowserWindow.getFocusedWindow();
  if (!win) return;

  const current = win.webContents.getZoomLevel();
  let next = current + delta;
  next = Math.max(-5, Math.min(5, next));
  win.webContents.setZoomLevel(next);
});
