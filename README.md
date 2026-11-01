<div align="center">
	<img src="https://lolisafe.moe/DJwzPbWD.png" />
</div>
<h2 align="center">Official LISTEN.moe Desktop Client</h2>
<h4 align="center">Simple, native <a href="https://listen.moe">LISTEN.moe</a> client for Windows/Linux/Mac, built with Rust and GPUI Kit.</h4>

https://github.com/user-attachments/assets/30162564-c47f-4d8a-aacf-9b5fb5b9a1ec

Drag any empty space to move the window. Click the cogwheel button or rightclick the app to configure various options as described below.

## Installation
Download and extract the latest release from [here](https://github.com/LISTEN-moe/desktop-app/releases), then run `listen-moe` (`listen-moe.exe` on Windows).

To build from source, install Rust 1.99 or newer and run:

```sh
cargo run --locked --release
```

Linux needs a Vulkan-capable graphics driver and the distribution equivalents of:

```sh
sudo apt-get install build-essential pkg-config libasound2-dev libdbus-1-dev \
  libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libx11-xcb-dev libxcb1-dev libx11-dev libegl1-mesa-dev libvulkan-dev
```

macOS needs Xcode Command Line Tools. Windows needs the MSVC Rust toolchain, Visual Studio C++ build tools, Windows SDK, and CMake.

## Features
* Scroll on volume icon to change the value, or click on it to show the bar
* Log in to favorite songs right from your desktop
* Easily switch between JPOP and KPOP in preferences
* Supports 2FA if you have it enabled on your account
* Supports native media controls and media display
* Display album art in both the app and the media controls
* Clicking on artist name or album art will open a tab on your browser with that artist/album page
* Toggle preferring romaji names over native japanese spellings
* Toggle the player to be always on top (controlled by the compositor on Wayland)
* Toggle small/big album art
* Audio visualization with colors drawn from the album art
* Optional Discord presence and event notifications

## Notes
This is still in development. Stuff might break.

## Author

**desktop-app** © [iCrawl](https://github.com/iCrawl).
Authored and maintained by iCrawl.

> GitHub [@iCrawl](https://github.com/iCrawl)
