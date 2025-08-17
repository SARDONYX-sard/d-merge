import { BrowserWindow, Menu, MenuItem, MenuItemConstructorOptions } from 'electron';

const templateMenu = [
  { role: 'undo' }, // Cmd/Ctrl + Z
  { role: 'redo' }, // Cmd/Ctrl + Shift + Z
  { type: 'separator' },
  { role: 'cut' }, // Cmd/Ctrl + X
  { role: 'copy' }, // Cmd/Ctrl + C
  { role: 'paste' }, // Cmd/Ctrl + V
  { role: 'delete' }, // Del
  { type: 'separator' },
  { role: 'selectAll' }, // Cmd/Ctrl + A
  { type: 'separator' },
  {
    label: 'reload',
    accelerator: 'CmdOrCtrl+R',
    click: (_item, focusedWindow) => {
      if (focusedWindow instanceof BrowserWindow) {
        focusedWindow.reload();
      }
    },
  },
  {
    label: 'Dev Tools(F12)',
    accelerator: 'F12',
    click: (_item, focusedWindow) => {
      if (focusedWindow instanceof BrowserWindow) {
        focusedWindow.webContents.openDevTools({ mode: 'detach' });
      }
    },
  },
] as const satisfies Array<MenuItemConstructorOptions | MenuItem>;

export const menu = Menu.buildFromTemplate(templateMenu);
