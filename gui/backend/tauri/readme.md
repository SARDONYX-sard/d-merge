# GUI Backend

## Permission

- [main.json](./capabilities/main.json) is permission config.
  The json completion (schema) of the permission is automatically created in `gen` dir when `npm run build` is done, so it is necessary to permit the backend API to be used in the frontend based on this.

  Note that we cannot use the backend API unless we allow it.

  See: [capability](https://v2.tauri.app/develop/resources/#accessing-files-in-javascript)
