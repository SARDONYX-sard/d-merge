// SPDX-FileCopyrightText: (C) 2019-2022, The Tauri Programme in the Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

declare namespace Tauri {
  /**
   * Extension filters for the file dialog.
   *
   * @since tauri 2.0.0
   */
  interface DialogFilter {
    /** Filter name. */
    name: string;
    /**
     * Extensions to filter, without a `.` prefix.
     * @example
     * ```typescript
     * extensions: ['svg', 'png']
     * ```
     */
    extensions: string[];
  }

  /**
   * Options for the open dialog.
   *
   * @since 2.0.0
   */
  interface OpenDialogOptions {
    /** The title of the dialog window (desktop only). */
    title?: string;
    /** The filters of the dialog. */
    filters?: DialogFilter[];
    /**
     * Initial directory or file path.
     * If it's a directory path, the dialog interface will change to that folder.
     * If it's not an existing directory, the file name will be set to the dialog's file name input and the dialog will be set to the parent folder.
     *
     * On mobile the file name is always used on the dialog's file name input.
     * If not provided, Android uses `(invalid).txt` as default file name.
     */
    defaultPath?: string;
    /** Whether the dialog allows multiple selection or not. */
    multiple?: boolean;
    /** Whether the dialog is a directory selection or not. */
    directory?: boolean;
    /**
     * If `directory` is true, indicates that it will be read recursively later.
     * Defines whether subdirectories will be allowed on the scope or not.
     */
    recursive?: boolean;
    /** Whether to allow creating directories in the dialog. Enabled by default. **macOS Only** */
    canCreateDirectories?: boolean;
  }

  /**
   * Options for the save dialog.
   *
   * @since tauri 2.0.0
   */
  interface SaveDialogOptions {
    /** The title of the dialog window (desktop only). */
    title?: string;
    /** The filters of the dialog. */
    filters?: DialogFilter[];
    /**
     * Initial directory or file path.
     * If it's a directory path, the dialog interface will change to that folder.
     * If it's not an existing directory, the file name will be set to the dialog's file name input and the dialog will be set to the parent folder.
     *
     * On mobile the file name is always used on the dialog's file name input.
     * If not provided, Android uses `(invalid).txt` as default file name.
     */
    defaultPath?: string;
    /** Whether to allow creating directories in the dialog. Enabled by default. **macOS Only** */
    canCreateDirectories?: boolean;
  }
}
