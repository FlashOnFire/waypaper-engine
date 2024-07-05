# 🎨 Waypaper Engine

Waypaper Engine is a Linux port of the popular Windows app Wallpaper Engine.  
Customize your desktop with beautiful, animated wallpapers created by the community.

## ✨ Features

- 🖼️ Use Wallpaper Engine on Linux !
- 🌐 Access a vast library of community-made wallpapers from the Wallpaper Engine workshop
- ⚡ Written in Rust, lightweight and blazingly fast
- 🔧 Easy to configure and use

## 📋 Requirements

- Rust >1.79
- Mpv
- A wayland compositor supporting the wlr_layer_shell protocol (tested on Hyprland and KWin)


To use this app you will also need an actual copy of Wallpaper Engine which you can buy and install from the official linux Steam client.

## 🛠️ Installation

WIP

## 🚀 Usage

The app is split in two binaries : the service (daemon) and the ui.
The daemon is the one setting up the wallpaper and will live on its own when you close the UI.  
As such, you don't need to keep Waypaper Engine Window open, and you can save system resources !

To start the daemon, use
```bash
cargo run --bin waypaper_engine_daemon --release
```

To start the UI, use
```bash
cargo run --bin waypaper_engine_ui --release
```

Make sure to start the daemon BEFORE the UI.

### Configuration

WIP

##

## 🤝 Contributing

Contributions are welcome! If you'd like to contribute, just make a PR.

## 📬 Contact

For any questions or feedback, feel free to fill a GitHub issue.