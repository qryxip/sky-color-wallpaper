# sky-color-wallpaper

[![CI](https://github.com/qryxip/sky-color-wallpaper/workflows/CI/badge.svg)](https://github.com/qryxip/sky-color-wallpaper/actions?workflow=CI)
![Maintenance](https://img.shields.io/maintenance/yes/2019)
![license](https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue)
[![dependency status](https://deps.rs/repo/github/qryxip/sky-color-wallpaper/status.svg)](https://deps.rs/repo/github/qryxip/sky-color-wallpaper)

Set random wallpapers according to sky color.

Inspired by [`sky-color-clock.el`](https://github.com/zk-phi/sky-color-clock).

## Supported platforms

- Windows
- macOS
- Linux
    - Gnome
    - KDE
    - Cinnamon
    - Unity
    - Budgie
    - XFCE
    - LXDE
    - MATE
    - Deepin
    - i3
    - xmonad
    - bspwm

## Installation

`sky-color-wallpaper` is not yet uploaded to [crates.io](https://crates.io).

```
$ cargo install --git https://github.com/qryxip/sky-color-wallpaper
```

## Usage

First, put a `sky_color_wallpaper.yml` in the [config directory](https://docs.rs/dirs/2/dirs/fn.config_dir.html).

```yaml
---
longitude: 135.0
latitude: 35.0

midnight:
  - ~/Pictures/wallpapers/sky_color_wallpaper/midnight/*
morning:
  - ~/Pictures/wallpapers/sky_color_wallpaper/morning/*
early_afternoon:
  - ~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/*
late_afternoon: # [sunset - 90min, sunset)
  - ~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/*
evening:
  - ~/Pictures/wallpapers/sky_color_wallpaper/evening/*
```

And run `sky-color-wallpaper`(`.exe`) at the startup.
On Windows, put the `exe` in `%USERPROFILE%\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup`.

## License

Licensed under <code>[MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](http://www.apache.org/licenses/LICENSE-2.0)</code>.
