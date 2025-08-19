// SPDX-FileCopyrightText: (C) 2023 DarkGuy10
// SPDX-License-Identifier: Apache-2.0 OR MIT

import { app, BrowserWindow, ipcMain, session } from 'electron';
import { menu } from './context_menu';
import { handleAccessRequest } from './url_resolver';
import { createWindow } from './window';
import './cmd'; // load ipc handlers

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// App event handlers

app.on('ready', () => {
  createWindow();
  session.defaultSession.webRequest.onBeforeRequest(handleAccessRequest);
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// backend functions for frontend: Need `preload.ts` settings

ipcMain.handle('show-context-menu', () => {
  menu.popup();
});

ipcMain.handle('zoom', (_event, delta: number) => {
  const win = BrowserWindow.getFocusedWindow();
  if (!win) return;

  const current = win.webContents.getZoomLevel();
  let next = current + delta;
  next = Math.max(-5, Math.min(5, next));
  win.webContents.setZoomLevel(next);
});
