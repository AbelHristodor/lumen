# Changelog

## v2.0.0

SDA Manager is now **Lumen**. This is a full rewrite with a new look, a new
name, and many new features. Your settings, themes, and hymns carry over.

### New name and logo

- The app is called Lumen. It has a new icon and a new wordmark.
- The program file is now `lumen`, or `lumen.exe` on Windows. It used to be
  `hymnal-gui`.
- The project moved to [AbelHristodor/lumen](https://github.com/AbelHristodor/lumen).
  Old links to `sda_manager` still redirect.

### A new app

The app is rebuilt in Rust with Iced and a GPU renderer. One window holds
five tabs. A Live strip stays in view on every tab and shows what the
projector shows.

- **Present.** Search hymns by number, title, file name, or a line of the
  lyrics. Search is fuzzy and ignores accents, so `plecati` finds
  *Plecaţi-vă*. See recent hymns, preview a slide, and go live.
- **Bible.** Pick a version, browse books and chapters, or type a reference
  like `John 3:16` or `Ioan 3,16`. Search every verse. Send verses to the
  screen a few at a time.
- **Videos.** Paste a link, pick a quality, and download. See progress and
  history.
- **Themes.** Edit a theme with a live preview. Pick fonts, sizes, colors,
  shadows, and outlines. Use a solid, gradient, particle, image, or video
  background. Pick a transition: fade, slide, scale, or blur. Mark
  favorites.
- **Settings.** Manage hymn folders, pick the projector screen, change the
  language, and check for updates.

### Projecting

- The projector opens full screen on the display you choose. Identify shows
  which screen is which.
- Slides that look wrong when parsed can be shown **as is**. Lumen turns the
  file into images with LibreOffice or PowerPoint, if one is installed.
- Text sizes itself to fit the slide.
- If the computer has no usable GPU, Lumen shows a notice.

### Keyboard

| Key | Action |
|---|---|
| Cmd/Ctrl+K | Search |
| Cmd/Ctrl+O | Projector on or off |
| Cmd/Ctrl+1 to 5 | Switch tab |
| Up and Down | Move the selection |
| Enter | Go live with the preview |
| Left, Right, Space | Previous or next live slide |
| B | Blank the screen |
| 1 to 9 | Jump to slide N |
| Esc | Stop the projector |

### Downloads and platforms

- Builds for macOS on Apple Silicon and Intel, Windows x64, and Linux x64
  and arm64.
- On macOS the download also has **Lumen.app** with its icon. The installer
  puts it in `~/Applications`.
- The program is about a third smaller than before.
- The hymn library downloads on first run. Later it updates only when it
  changes.
- English, Italian, and Romanian.

### Upgrading from 1.x

- **Self update works.** A 1.x install finds 2.0.0 and updates itself. It
  keeps its old file name until you reinstall.
- **Or reinstall** with the commands in the [README](README.md#install).
  The installers remove the old `hymnal-gui` program, its folder, its PATH
  entry, and the old desktop shortcut.
- Settings, themes, history, and downloaded hymns stay where they are.
- The app is still unsigned. On macOS, right click it and choose **Open**
  the first time.
