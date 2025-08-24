# Backend API

- You can call functions where `#[tauri::command]` is used.

## Rules

- All functions that connect backends should be written in this dir.
- Backend APIs are used only through functions in this dir.

# Common mistakes with Electron

With tauri, it is okay to have extra arguments sent to the backend, but with Electron, trying to pass function-based options to the backend will result in an error.

```js
const includeInvalidOpts = {
  fn: ()=> {} // Electron does not allow functions to be passed!
  validOpt: ""
};
backendApi(includeInvalidOpts); // ERR!: An object could not be cloned
```

When summarizing options such as the above, be careful to pass only valid objects to the Electron API.
