# egui_code_editor

A small, language-agnostic code editor widget for [egui](https://github.com/emilk/egui):
syntax highlighting, diagnostics, hover, completion, signature help, inlay hints, formatting
(all provided by a `Language` implementation) and two key binding styles (`Keymap::Normal`,
`Keymap::Vim`).

```rust
let mut state = CodeEditorState::default(); // keep one per document

let output = CodeEditor::new("my_editor", &mut text)
    .language(&MyLanguage)
    .diagnostics(&diagnostics)
    .keymap(Keymap::Vim)
    .line_numbers(true) // default
    .show(ui, &mut state);

// Optional: the keymap status line, e.g. `[ NORMAL ]  recording @a` (colored by mode).
egui_code_editor::status_line(ui, &state, Keymap::Vim);
```

## Editor keys (all keymaps)

| Key                  | Action                     |
| -------------------- | -------------------------- |
| `Ctrl+Space`         | Open completion            |
| `Tab` / `Enter`      | Accept completion          |
| `Shift+Alt+F`        | Format document            |
| Mouse rest on a word | Hover (diagnostics + docs) |

## Vim

`Keymap::Vim` emulates the commonly used part of Vim. Settings: `KeymapState::vim_options_mut`
(`VimOptions`: Insert-mode escape sequences such as `jk`, `shift_width`).

`[count]` can precede most commands (e.g., `3j`, `2dd`), and `"x` selects a register
(e.g., `"ayy`, `"ap`).

### Modes

| Key                     | Action                                                 |
| ----------------------- | ------------------------------------------------------ |
| `i` `a` `I` `A`         | Insert before / after cursor, line start / end         |
| `gi` / `gI`             | Insert where Insert mode was left / at column 0        |
| `o` / `O`               | Open a line below / above                              |
| `v` / `V` / `Ctrl+V`    | Visual / Visual Line / Visual Block                    |
| `gv`                    | Reselect the last Visual selection                     |
| `Esc` `Ctrl+[` `Ctrl+C` | Back to Normal mode                                    |
| `jk` `kj` `jj` (Insert) | Back to Normal mode (typed within 0.5 s, configurable) |
| `:` / `/` / `?`         | Command line / search forward / backward               |

### Motions

| Key                                     | Moves to                                                               |
| --------------------------------------- | ---------------------------------------------------------------------- |
| `h` `j` `k` `l`, arrows                 | Left, down, up, right                                                  |
| `Space` / `Backspace`                   | Right / left                                                           |
| `Ctrl+N` / `Ctrl+P`                     | Down / up                                                              |
| `w` `b` `e` / `W` `B` `E`               | Next word start, previous word start, word end (WORD: blank separated) |
| `ge` / `gE`                             | Previous word end                                                      |
| `0` `^` `$`, `Home` / `End`             | Line start, first non-blank, line end                                  |
| `g0` `g^` `g$` `gj` `gk`                | Same as `0` `^` `$` `j` `k` (lines never wrap)                         |
| `\|`                                    | Column `[count]`                                                       |
| `+` `Enter` / `-` / `_`                 | First non-blank of next / previous / `[count]-1` lines down            |
| `gg` / `G`                              | First / last line (`[count]`: that line)                               |
| `{` / `}`                               | Previous / next paragraph                                              |
| `%`                                     | Matching `()` `[]` `{}`                                                |
| `f<c>` `F<c>` / `t<c>` `T<c>`           | Char `c` on the line (forward / backward), till before it              |
| `;` / `,`                               | Repeat the last `f` `t` `F` `T` / in reverse                           |
| `H` `M` `L`                             | Top / middle / bottom of the view                                      |
| `Ctrl+D` / `Ctrl+U`                     | Half a page down / up                                                  |
| `Ctrl+F` `PageDown` / `Ctrl+B` `PageUp` | A page down / up                                                       |
| `n` / `N`                               | Next / previous search match                                           |
| `'<a-z>` / `` `<a-z> ``                 | Mark line / exact position                                             |
| `''` / ` `` `                           | Position before the latest jump                                        |

### Operators

Used as `{operator}{motion}`, `{operator}{text object}` or doubled for whole lines
(e.g., `dw`, `ci(`, `dd`, `>>`, `gUU`). In Visual mode they act on the selection.

| Key         | Action                       |
| ----------- | ---------------------------- |
| `d`         | Delete                       |
| `c`         | Change (delete + Insert)     |
| `y`         | Yank (also to the clipboard) |
| `>` / `<`   | Indent / outdent lines       |
| `gu` / `gU` | Lowercase / uppercase        |
| `g~`        | Toggle case                  |

### Text objects

`i` = inner, `a` = around (with the delimiters / trailing blanks), e.g. `diw`, `ca"`, `yap`.

| Key                   | Object                      |
| --------------------- | --------------------------- |
| `iw` `aw` / `iW` `aW` | Word / WORD                 |
| `ip` `ap`             | Paragraph                   |
| `i"` `i'` `` i` ``    | Quoted string (on the line) |
| `i(` `i)` `ib`        | `( )` block                 |
| `i[` `i]`             | `[ ]` block                 |
| `i{` `i}` `iB`        | `{ }` block                 |
| `i<` `i>`             | `< >` block                 |

### Editing

| Key                 | Action                                                        |
| ------------------- | ------------------------------------------------------------- |
| `x` `Delete` / `X`  | Delete char under / before the cursor                         |
| `D` / `C` / `Y`     | Delete / change to line end, yank line                        |
| `s` / `S`           | Change char / whole line                                      |
| `r<c>`              | Replace char(s) with `c`                                      |
| `~`                 | Toggle case of char(s)                                        |
| `J` / `gJ`          | Join lines with / without a space                             |
| `p` / `P`           | Paste after / before (linewise, charwise or block)            |
| `Ctrl+A` / `Ctrl+X` | Add / subtract `[count]` to the number at or after the cursor |
| `u` / `Ctrl+R`      | Undo / redo                                                   |
| `.`                 | Repeat the last change (including typed text)                 |

### Insert mode

| Key                 | Action                                   |
| ------------------- | ---------------------------------------- |
| `Ctrl+W`            | Delete the word before the cursor        |
| `Ctrl+U`            | Delete from the line start to the cursor |
| `Ctrl+T` / `Ctrl+D` | Indent / outdent the line                |

### Visual mode

| Key                     | Action                                         |
| ----------------------- | ---------------------------------------------- |
| `d` `x` / `y` / `c` `s` | Delete / yank / change the selection           |
| `S`                     | Change the selected lines                      |
| `>` `<` / `~` `u` `U`   | Shift lines / change case                      |
| `J`                     | Join the selected lines                        |
| `p` `P`                 | Replace the selection with the register        |
| `I` / `A`               | Insert at the start / append at the end        |
| `o` / `O`               | Go to the other end of the selection           |
| `:`                     | Command line for the selected lines (`:'<,'>`) |

### Visual Block mode (`Ctrl+V`)

| Key           | Action                                                   |
| ------------- | -------------------------------------------------------- |
| `d` `x` / `y` | Delete / yank the block (`p` pastes it as a block)       |
| `c` `s`       | Change: the text typed is applied to every line on `Esc` |
| `I` / `A`     | Insert before / append after the block on every line     |
| `r<c>`        | Replace every char of the block                          |
| `~` `u` `U`   | Change case                                              |
| `>` `<`       | Shift the lines                                          |
| `o` / `O`     | Other corner / other side on the same line               |

### Search

Plain text (no regex). Smartcase: a pattern without uppercase letters ignores case.
All matches are highlighted; the status line shows the match count (e.g., `/foo    [2/5]`).

| Key             | Action                                                         |
| --------------- | -------------------------------------------------------------- |
| `/pat` / `?pat` | Search forward / backward (`Enter` runs, empty = last pattern) |
| `n` / `N`       | Next / previous match (wraps around)                           |
| `*` / `#`       | Search the word under the cursor (whole word)                  |
| `g*` / `g#`     | Same, also inside longer words                                 |
| `:noh`          | Hide the highlight                                             |

### Marks, jumps, macros

| Key                       | Action                                  |
| ------------------------- | --------------------------------------- |
| `m<a-z>`                  | Set a mark                              |
| `Ctrl+O` / `Ctrl+I` `Tab` | Older / newer position in the jump list |
| `q<a-z>` ... `q`          | Record a macro                          |
| `@<a-z>` / `@@`           | Play a macro / the last one             |

### Scrolling

| Key                   | Action                                   |
| --------------------- | ---------------------------------------- |
| `zz` / `zt` / `zb`    | Cursor line to the center / top / bottom |
| `z.` / `z<CR>` / `z-` | Same, and go to the first non-blank      |
| `Ctrl+E` / `Ctrl+Y`   | Scroll the view down / up by a line      |

### Registers

| Register       | Contents                                                    |
| -------------- | ----------------------------------------------------------- |
| `""` (unnamed) | The last yank / delete (also copied to the clipboard)       |
| `"0`           | The last yank                                               |
| `"a`..`"z`     | Named; `"A`..`"Z` append                                    |
| `"+` `"*`      | System clipboard (write only; pasting uses the unnamed one) |
| `"_`           | Black hole                                                  |

### Other

| Key  | Action                                                                                                                           |
| ---- | -------------------------------------------------------------------------------------------------------------------------------- |
| `K`  | Hover at the cursor; `K` again moves the keyboard into it (`Tab` / `j` / `k` choose a link, `Enter` opens it, `Esc` / `q` close) |
| `gx` | Open the URL under the cursor                                                                                                    |

### Ex commands

`[range]`: `%` (all), `'<,'>` (last Visual selection), `n`, `n,m`, `.`, `$`.

| Command                   | Action                                      |
| ------------------------- | ------------------------------------------- |
| `:w` `:wq` `:x` `:up`     | Save (`EditorCommand::Save` for the host)   |
| `:<n>`                    | Go to line `n`                              |
| `:[range]s/pat/rep/[g]`   | Substitute (plain text; `g`: all in a line) |
| `:[range]sort[!] [u]`     | Sort lines (`!` reverse, `u` unique)        |
| `:[range]d` / `:[range]y` | Delete / yank lines                         |
| `:u` / `:red`             | Undo / redo                                 |
| `:reg`                    | Show the registers                          |
| `:delm {a-z}`             | Delete marks                                |
| `:noh`                    | Hide the search highlight                   |

The prompt line accepts `Backspace`, `Ctrl+U` (clear) and `Ctrl+W` (delete a word).

### Not supported

Regex search / substitution, Replace mode (`R`), `=` (auto indent), `gn`, `:set`, key mappings,
window / buffer commands.
