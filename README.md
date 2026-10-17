# CalcCraft

Calculator; an open-source reimplementation of Calculator, rebuilt in pure Rust with [eframe/egui](https://github.com/emilk/egui).
It looks like the original (Segoe UI fonts, Fluent icons, the same layout and colors) and behaves like it, down to how the expression line, `%`, repeated `=` and error messages work.

| Dark | Light |
| --- | --- |
| ![The calculator in the dark theme](docs/screenshot.png) | ![The calculator in the light theme](docs/screenshot-light.png) |

## Features

- **Keypad:** digits, decimal point, `+/−`, `+ − × ÷`, `=`, `%`, `CE`, `C`, `⌫`, `1/x`, `x²`, `√x`.
- **Memory:** `MC`, `MR`, `M+`, `M−`, `MS`, plus the memory list (`M˅`) with per-slot `MC` / `M+` / `M−`,
  also in each slot's right-click menu.
- **History:** every calculation is logged. Click an entry to bring it back, right-click it to copy
  or delete it, or use the trash button to clear the list.
- **Docked history and memory:** from 560 px wide, history and memory sit next to the keypad
  in a side panel with History and Memory tabs, as in Windows; narrower windows open them over
  the keypad.
- **Keep on top:** a compact always-on-top window, the same as in Windows.
- **Settings:** opened from the menu, as in Windows. It has the App theme choice, the Language and
  Regional format choices, the app version, and links for sending feedback and to the project on
  GitHub.
- **Light and dark themes:** pick Light, Dark or Use system setting in Settings. With the system
  setting, the app follows the Windows app mode, even when it changes while the app is open.
  The title bar switches with it. The choice is saved in `%APPDATA%\CalcCraft`.
- **Exact arithmetic:** numbers are stored as big rationals, so `0.1 + 0.2 − 0.3` is exactly `0`
  and `1 ÷ 3 × 3` is exactly `1`.
- **Windows behavior:** immediate execution without operator precedence (`2 + 3 × 4 = 20`),
  16 significant digits, Windows' rules for scientific notation, and the same error messages
  (`Cannot divide by zero`, `Result is undefined`, `Invalid input`, `Overflow`).
- **Keyboard and clipboard:** see the shortcuts below. `Ctrl+C` copies the result. `Ctrl+V` pastes
  a number (`1.5e3` too) or a whole expression such as `12 + 5 =`, with the same checks as Windows.
- **Keyboard navigation:** `Tab`, `Shift+Tab` and the arrow keys reach every control, with
  Windows' focus rectangle, and `Space` or `Enter` presses it (`Enter` stays `=` on the keypad).
  Menus, lists and the calendar open on their selected item and hand the focus back when they
  close.
- **Screen readers:** Narrator reads every control by its Windows name ("Positive negative",
  "Memory store", …) and announces results as they change ("Display is 7").
- **50 languages:** the interface uses the Windows Calculator's own translations and follows the
  Windows display language. Numbers, dates and the first day of the week follow your regional
  format (`1.234,5` and `15. Oktober 2026` in Germany), as in Windows. Unlike Windows, Settings
  can also set the language and the regional format for CalcCraft alone: type the first letters
  of a name to find it in the list.
- **Scientific mode** (open the ☰ menu, or `Alt+2`):
  - Operator precedence and parentheses (`2 + 3 × 4 = 14`), with the open count on `(`.
  - Powers and roots (`x²`, `x³`, `xʸ`, `10ˣ`, `2ˣ`, `eˣ`, `√x`, `∛x`, `ʸ√x`), `log`, `ln`,
    `logᵧx`, `n!` (the gamma function for fractions: `0.5!` is `√π / 2`), `|x|`, `⌊x⌋`, `⌈x⌉`,
    `mod`, `π`, `e` and `rand`; `2nd` shows the second functions.
  - The Trigonometry menu: `sin`, `cos`, `tan`, `sec`, `csc`, `cot`, their inverses (`2nd`) and
    hyperbolic versions (`hyp`), in degrees, radians or gradians. The Function menu adds
    `→dms` and `→deg` for degrees, minutes and seconds.
  - `exp` types an exponent (`1.5e+3`), and `F-E` switches to scientific notation.
  - 32 significant digits, as in Windows (`3.1415926535897932384626433832795`), Windows'
    error rules (`tan(90°)` is `Invalid input`) and expression line (`sin₀(30)`, `2 ^ 10 =`).
  - Its own history; memory is shared with the other modes.

![Screenshot of the scientific mode](docs/scientific.png)

- **Graphing mode** (open the ☰ menu, or `Alt+3`):
  - Up to 14 equations at once, each in its own color: functions (`y = x^2`, or just `x^2`),
    `x = …`, relations such as `x^2 + y^2 = 25`, and inequalities (`y < sin(x)`), whose region
    is dotted and whose strict boundary is dashed. Type them, or use the keypad with `2nd` and
    its Trigonometry, Inequalities and Function menus; implicit multiplication (`2x`, `mx + b`)
    works. An equation is graphed when you press `Enter` or leave its box.
  - The colored fₙ button hides and shows a line; hover a box for the function analysis, the
    line options (14 colors, solid, dashed or dotted) and remove.
  - Pan by dragging, zoom with the wheel, the + and − buttons or `Ctrl` + plus and minus. The
    view fits the equations by itself until you move it; the graph view button (`Ctrl+0`)
    brings that back.
  - Hover a curve to read a point, or turn on tracing and move its pointer with the arrow keys.
  - Free variables get sliders: `y = mx + b` gets `m` and `b`, each with its minimum, step and
    maximum.
  - **Function analysis:** domain, range, intercepts, minima and maxima, inflection points,
    asymptotes, parity, period and monotonicity, with exact values (`(√3/3 + 1, −2√3 − 6)`,
    `{(2πn₁ + π/2, 1), n₁ ∈ ℤ}`).
  - **Graph options:** the window's x and y ranges, radians, degrees or gradians, the line
    thickness, and the graph theme (always light, as in Windows, or matching the app).
  - Right-click the graph to copy it as an image, or use the share button to send the graph
    with its equations and variables through the Windows share sheet.

- **Date calculation mode** (open the ☰ menu):
  - **Difference between dates:** the difference broken into years, months, weeks and days,
    plus the total number of days (`4 years, 4 months, 4 days`, `1587 days`).
  - **Add or subtract days:** add years, months and days (0–999 each) to a date, or subtract them.
  - The date pickers open a calendar with month, year and decade views. It works with the
    arrows and the mouse wheel, and supports dates from 1601 to 9999, like Windows.

![Screenshot of the date calculation mode](docs/date-calculation.png)

- **Programmer mode** (open the ☰ menu, or `Alt+4`):
  - The value in HEX, DEC, OCT and BIN at once; click a radix to type in it. Digits that
    don't exist in the radix are disabled, and right-clicking a radix copies its value.
  - QWORD, DWORD, WORD and BYTE words in two's complement: a smaller word keeps the low
    bits, a larger one extends the sign, and decimal numbers are signed (`FF` is `-1` in a byte).
  - `AND`, `OR`, `NOT`, `NAND`, `NOR`, `XOR` (the Bitwise menu), shifts and rotations (the Bit
    shift menu: arithmetic, logical, circular, or through the carry bit), integer `÷` and the
    remainder `%`, all with operator precedence and parentheses, as in Windows.
  - The bit toggling keypad shows all 64 bits; click one to flip it.
  - The engine is Windows' own integer mode, quirks included: switching the radix rewrites
    the expression line, and a quotient inside parentheses keeps its fraction until they
    close (`(255 AND 10 − 7 ÷ 2)` is 6).
  - Memory is shared with the Standard mode, as in Windows; there is no history.

![Screenshot of the programmer mode](docs/programmer.png)

- **Unit converters** (under Converter in the ☰ menu): Volume, Length, Weight and mass,
  Temperature, Energy, Area, Speed, Time, Power, Data, Pressure and Angle.
  - Windows' units and conversion factors, and its rounding (at least 7 significant digits).
  - Type into either value; click the other one to convert the other way.
  - "About equal to" shows the value in other units and an everyday comparison, like
    `5.36 hands` for a meter.
  - The default units follow your Windows region, as in Windows (inches and pounds in the US,
    centimeters and kilograms elsewhere). The last converter's units are remembered.

![Screenshot of the Length converter](docs/unit-converter.png)

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `0`–`9`, `.` or `,` | Digits and decimal point |
| `+` `-` `*` `/` | Operators |
| `Enter` or `=` | Equals |
| `%` | Percent |
| `@` | Square root |
| `R` | Reciprocal (`1/x`) |
| `Q` | Square (`x²`) |
| `F9` | Negate (`+/−`) |
| `Backspace` | Delete the last digit |
| `Delete` | Clear entry (`CE`) |
| `Esc` | Clear (`C`), close an open menu or panel, or leave Settings |
| `Ctrl+M` / `Ctrl+P` / `Ctrl+Q` / `Ctrl+R` / `Ctrl+L` | `MS` / `M+` / `M−` / `MR` / `MC` |
| `Ctrl+H` | Show or hide history (in narrow windows) |
| `Ctrl+Shift+D` | Clear history |
| `Ctrl+C` / `Ctrl+V` | Copy the result / paste a number or an expression |
| `Alt+1` / `Alt+2` / `Alt+3` / `Alt+4` / `Alt+5` or `Ctrl+E` | Standard / Scientific / Graphing / Programmer / Date calculation mode |
| `Tab` / `Shift+Tab`, arrow keys | Move the keyboard focus |
| `Space` or `Enter` | Press the focused control (`Enter` is `=` on the keypad) |

In the date calculation mode, `Esc` closes the open calendar or list, `Page Up` and `Page Down`
flip the calendar's pages, and `Ctrl+C` copies the result.

In the scientific mode, these keys are added (Windows' own):

| Key | Action |
| --- | --- |
| `S` / `O` / `T` / `U` / `I` / `J` | `sin` / `cos` / `tan` / `sec` / `csc` / `cot` |
| `Shift` + those keys | The inverse functions (`sin⁻¹` …) |
| `Ctrl` + those keys, `Ctrl+Shift` + those keys | The hyperbolic functions (`sinh` …) and their inverses |
| `^` or `Y`, `Ctrl+Y` | `xʸ`, `ʸ√x` |
| `Q`, `#`, `@`, `B` | `x²`, `x³`, `√x`, `∛x` |
| `Ctrl+G`, `G`, `Ctrl+N` | `10ˣ`, `2ˣ`, `eˣ` |
| `L`, `N`, `Shift+L` | `log`, `ln`, `logᵧx` |
| `!`, `\|`, `[`, `]` | `n!`, `\|x\|`, `⌊x⌋`, `⌈x⌉` |
| `%` | `mod` |
| `P`, `Shift+E`, `Shift+R` | `π`, `e`, `rand` |
| `X` or `E`, `V` | `exp`, `F-E` |
| `M` | `→dms` |
| `R` | Reciprocal (`1/x`) |
| `F4` / `F5` / `F3` | Radians / gradians / degrees (as in Windows, each is the key of the unit button before it) |
| `(` `)` | Parentheses |

In the programmer mode, Windows' own keys apply:

| Key | Action |
| --- | --- |
| `0`–`9`, `A`–`F` | Digits (`A`–`F` in HEX only) |
| `F5` / `F6` / `F7` / `F8` | HEX / DEC / OCT / BIN |
| `F2` / `F3` / `F4` / `F12` | DWORD / WORD / BYTE / QWORD |
| `&` `\|` `^` `~` | AND, OR, XOR, NOT |
| `.` `\` | NAND, NOR |
| `<` `>` | Shift or rotate left and right, as chosen in the Bit shift menu |
| `%` | Remainder (`Mod`) |
| `(` `)` | Parentheses |

`Ctrl+C` copies the value in the current radix without its digit groups, and `Ctrl+V` pastes
numbers and expressions in the current radix, C-style notation included (`0x1F` or `1Fh` in HEX,
`0b101` in BIN, `255u` or `1_000` in DEC), as long as they fit into the word size.

In the graphing mode, the keyboard types into the equation box with the focus, and:

| Key | Action |
| --- | --- |
| `Enter` | Graph the equation and go to the next box |
| `Ctrl` + `+` or `=`, `Ctrl` + `-` | Zoom in, zoom out |
| `Ctrl+0` | The graph view button: back to the automatic view, or keep the current one |
| Arrow keys (with `Shift`: slower) | Move the trace pointer while tracing |
| `Esc` | Stop tracing, or close an open menu or flyout |

In the converters, the digits, `.` or `,`, `Backspace` and `Delete` (`CE`) work as above,
`F9` or `-` changes the sign (Temperature, Power and Angle only), `Ctrl+C` copies the active
value, `Ctrl+V` pastes a number, and `Esc` closes an open unit list.

In an open unit list or a focused unit picker, typing the first letters of a unit's name goes
to that unit, as in Windows.

## Requirements

- **Windows 10 or 11.** The app was developed and tested on Windows 11. It loads Segoe UI
  (regular, semibold and light) and Segoe UI Symbol from `%WINDIR%\Fonts`. Its icons
  ([Fluent System Icons](https://github.com/microsoft/fluentui-system-icons)) are built into the
  executable, and so is [Inter](https://fonts.google.com/specimen/Inter), which takes Segoe UI's
  place where it's missing.
- **Linux and macOS (x64)** get release builds too, with Inter, a plain title bar and en-US
  regional formats, but Windows is the platform CalcCraft is made for. The Linux build needs X11 or
  Wayland and OpenGL, as any desktop has. The macOS app isn't notarized: open it the first time
  with right-click → Open (or remove the quarantine flag with `xattr -dr com.apple.quarantine
  CalcCraft.app`).
- **Rust 1.95 or newer** (edition 2024; eframe 0.36 needs at least 1.95).
- **Windows SDK** (on Windows) for `rc.exe`, which embeds the icon into the executable. It comes with the
  Visual Studio C++ build tools that Rust on Windows needs anyway.
- **On Linux,** eframe's build dependencies:
  `sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev`.

## Build and run

```bash
cargo run --release
```

The executable is `target/release/calculator.exe` (`calculator` on Linux and macOS). Release
builds use the Windows GUI subsystem, so no console window appears. Debug builds (`cargo run`) keep the console, which is
handy for logging.

## Development

```bash
cargo test                                  # calculator and date tests
cargo fmt --check                           # formatting
cargo clippy --all-targets -- -D warnings   # lints
```

All three must pass before a change is merged.
[GitHub Actions](.github/workflows/ci.yml) runs them on Windows for every push and pull request.
Contributors (people and AI agents alike)
should read [AGENTS.md](AGENTS.md) first: it describes the architecture, the Windows behaviors
that the tests pin down, and known pitfalls.

Besides the Language and Regional format settings, `CALCCRAFT_LANGUAGE` and `CALCCRAFT_REGION`
(for example `de-DE`) stand in for the Windows settings that "Use system setting" follows, which
helps testing. The strings in `locales/` come from the Windows Calculator's resource files;
after adding a string to `tools/import-strings.ps1`, regenerate them from a clone of
[microsoft/calculator](https://github.com/microsoft/calculator):

```bash
pwsh tools/import-strings.ps1 -Source ../win-calculator
```

The few strings Windows doesn't have (the Language and Regional format settings) are
CalcCraft's own, translated by hand in `locales/calccraft.txt`.

## Releasing

Bump `version` in `Cargo.toml`, merge that change, then push a tag with the same version:

```bash
git tag v1.0.0
```

```bash
git push origin v1.0.0
```

The [release workflow](.github/workflows/release.yml) checks that the tag matches `Cargo.toml`
and creates a draft release with generated release notes. Then it runs the tests and builds the
app on Windows, Linux and macOS, all x64, and uploads each with its SHA-256 checksum:

| Platform | File |
| --- | --- |
| Windows | `calculator-v1.0.0-windows-x64.exe` |
| Linux | `calculator-v1.0.0-linux-x64.tar.gz` (the `calculator` executable) |
| macOS | `calculator-v1.0.0-macos-x64.zip` (`CalcCraft.app`, signed ad hoc, cross-compiled on Apple silicon) |

The draft is published only when all three are there. Tags with a suffix, such as
`v1.0.0-beta.1`, are published as pre-releases.

## Project layout

| Path | Contents |
| --- | --- |
| `src/main.rs` | Entry point: window options |
| `src/icon.rs` | The app icon, drawn in code: the window icon and the icon embedded in the executable |
| `build.rs` | Embeds the icon and the version information (CalcCraft, version from `Cargo.toml`) into `calculator.exe` |
| `src/engine.rs` | Calculator logic: a port of the Windows engine (immediate execution, plus operator precedence and parentheses for the Scientific and Programmer modes), exact arithmetic, number formatting, exponent entry, paste, memory, history, unit tests. No UI code. |
| `src/engine/math.rs` | The Scientific mode's functions: powers, roots, logarithms, trigonometry, hyperbolic functions, the gamma function, computed with a few hundred bits and Windows' domain rules; unit tests. No UI code. |
| `src/engine/integer.rs` | The Programmer mode's integer mode: radixes, word sizes, two's complement, bitwise operators, shifts and rotations, digit limits and paste rules; unit tests. No UI code. |
| `src/dates.rs` | Date logic: difference in years, months, weeks and days; adding and subtracting; date formats; unit tests. No UI code. |
| `src/converter.rs` | Unit converter logic: a port of the Windows converter with its units, factors, rounding and region defaults; paste; unit tests. No UI code. |
| `src/locale.rs` | Regional number formats (decimal separator, digit grouping) and date patterns; unit tests. No UI code. |
| `src/graphing.rs`, `src/graphing/` | The graphing mode's math: the equation parser (`parse.rs`), expressions and their derivatives (`expr.rs`), plotting, best fit and tracing (`plot.rs`), the function analysis (`analysis.rs`) and exact values (`exact.rs`); unit tests. No UI code. |
| `src/app.rs` | User interface: custom-painted buttons, display, memory row, history and memory panels (flyouts or docked) with their context menus, menu, Settings, keyboard handling, fonts, title bar tint |
| `src/app/access.rs` | Keyboard focus, focus rectangles and screen reader names for the custom-painted controls |
| `src/app/strings.rs` | The interface's text in the user's language, from `locales/` |
| `src/app/region.rs` | The user's regional formats, read from Windows |
| `src/app/palette.rs` | The dark and light color palettes |
| `src/app/icons.rs` | The icons: code points of the embedded icon font |
| `src/app/date_view.rs` | User interface of the date calculation mode: date pickers, calendar, drop-down lists |
| `src/app/converter_view.rs` | User interface of the converters: values, unit drop-downs, keypad |
| `src/app/scientific_view.rs` | User interface of the scientific mode: angle unit and F-E, Trigonometry and Function menus, keypad with `2nd` |
| `src/app/programmer_view.rs` | User interface of the programmer mode: radix list, word size, Bitwise and Bit shift menus, keypad, bit toggling keypad |
| `src/app/graphing_view.rs` | User interface of the graphing mode: layout, equation boxes, variable sliders, function analysis, graph options and line options |
| `src/app/graph_canvas.rs` | The graphing mode's graph: grid, axes, lines and regions, tracing, the buttons over the graph, copying it |
| `src/app/graphing_keypad.rs` | The graphing mode's keypad and its Trigonometry, Inequalities and Function menus |
| `assets/fonts/` | The fonts built into the executable: the icon font and Inter, with their licenses |
| `tools/icon-font.py` | Builds the icon font from Fluent System Icons (`uv run tools/icon-font.py`) |
| `locales/` | The strings in 60 languages, imported from the Windows Calculator by `tools/import-strings.ps1`, and CalcCraft's own few in `calccraft.txt` |
| `docs/screenshot.png`, `docs/screenshot-light.png`, `docs/scientific.png`, `docs/date-calculation.png`, `docs/programmer.png`, `docs/unit-converter.png` | The screenshots shown above |
| `.github/workflows/` | GitHub Actions: CI checks and the release build |
| `AGENTS.md` | Guide for contributors and AI coding agents |
| `CLAUDE.md` | Entry point for Claude Code (imports `AGENTS.md`) |

## Differences from Windows Calculator

- All modes and 12 of the 13 converters exist. The Currency converter is not implemented.
- The graphing mode has its own math engine (Windows' is proprietary):
  - Equations are plain text laid out like math (italic variables, raised exponents), not
    Windows' math editor, so a fraction stays `1/x` instead of being built up. Functions beyond
    the keypad's (Windows' `deriv`, `integral` …) aren't supported.
  - The function analysis is numeric, with the results written as the integers, fractions,
    multiples of π and square roots they are; other values are decimals (6 significant digits),
    where Windows' symbolic engine would give an exact form. Features beyond ±1000 aren't found.
  - The automatic view uses CalcCraft's own rules. They match Windows' view for `x^2`, and give
    similar ones elsewhere.
  - A share's title is "CalcCraft" instead of Windows' "Look what I graphed with Windows
    Calculator".
- The icons are Fluent System Icons built into the app, not Windows' Segoe Fluent Icons: they
  are the same designs in most places, but not every line is identical. Six converter icons in
  the menu (Volume, Weight and mass, Temperature, Speed, Data, Angle), the Programmer mode's icons
  and the Scientific mode's key labels (`x²`, `xʸ`, `2ⁿᵈ`, …) and menu icons are similar icons,
  Segoe UI text or drawn in code: Windows draws them from the Calculator's own icon font, which
  can't be used outside Windows.
- The Scientific mode computes its functions with about 120 digits and keeps 40, so the 32
  digits it shows are those of the exact result. Windows computes with about 32 digits, so some
  of its results may differ in the last digit.
- Dates always use the Gregorian calendar, even in regions that use another one.
- The palettes are fixed: both themes use the purple tint and accent color of the reference
  screenshots. Windows tints its window with the Mica effect and your own accent color.
  The light background is an estimate, not a sample.
- There is no high contrast theme.
- The About section in Settings shows only the name and version, without the license and
  privacy links.
- egui can't lay out right-to-left text or shape complex scripts, so Arabic, Persian, Hebrew,
  Hindi, Kannada, Malayalam, Tamil, Telugu, Khmer and Lao users get the next language in their
  Windows language list, or English. Numbers always use Western digits, and the interface is
  never mirrored.
- The menu button's screen reader names and the calendar's arrows stay English: Windows takes
  them from places outside the Calculator's own translations.
- Pasted text loses its group separators before the decimal separator becomes a point. Windows
  does it the other way round, which loses the decimal point where the group separator is a point
  (`1,5` pastes as `15` with German formats).
- `Esc` leaves the Settings page; in Windows only the back button does.

## Disclaimer

This is an independent hobby project. It is not affiliated with or endorsed by Microsoft.
Windows and Segoe are trademarks of Microsoft. The Segoe fonts are loaded from your Windows
installation and are not part of this repository.

## Acknowledgements & Credits

This project is heavily inspired by and builds upon the architecture and logic of the official [Microsoft Calculator](https://github.com/Microsoft/calculator) repository, licensed under the [MIT License](https://github.com/microsoft/calculator/blob/main/LICENSE). The translations in `locales/` are taken from its resource files.
- The icons are [Fluent System Icons](https://github.com/microsoft/fluentui-system-icons) by Microsoft, licensed under the MIT License.
- The fallback text font is [Inter](https://rsms.me/inter/) by Rasmus Andersson, licensed under the SIL Open Font License 1.1.

See [`assets/fonts/README.md`](assets/fonts/README.md) for the exact sources and license files.

> **Disclaimer:** This project is an independent, community-driven port to Rust and is **not** affiliated with, endorsed by, or in any way officially connected with Microsoft Corporation.

We gratefully acknowledge Microsoft and the contributors of the original project for their work.
