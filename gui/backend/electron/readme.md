# Developers Note

## Purpose of this code

The reason for creating an Electron app was to run it on MO2's virtual fs. Tauri sometimes crashes with Webview hooks (on a PC environment other than my own).

# Known Issues

Currently, there are the following bugs

- [ ] For some reason, “check all” on the frontend does not work
- [ ] After moving the Next.js path, reloading causes a crash
- [ ] Cannot start when managed with MO2.
