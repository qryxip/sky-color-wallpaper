# sky-color-wallpaper

[![CI](https://github.com/qryxip/sky-color-wallpaper/workflows/CI/badge.svg)](https://github.com/qryxip/sky-color-wallpaper/actions?workflow=CI)
![Maintenance](https://img.shields.io/maintenance/yes/2019)
[![Crates.io](https://img.shields.io/crates/v/sky-color-wallpaper)](https://crates.io/crates/sky-color-wallpaper)
[![Crates.io](https://img.shields.io/crates/l/sky-color-wallpaper)](https://crates.io/crates/sky-color-wallpaper)
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

### GitHub Releases

<https://github.com/qryxip/sky-color-wallpaper/releases>

### `cargo install` (crates.io)

```
$ cargo install sky-color-wallpaper
```

### `cargo install` (GitHub)

```
$ cargo install --git https://github.com/qryxip/sky-color-wallpaper
```

## Usage

First, put a `sky_color_wallpaper.yml` in the [config directory](https://docs.rs/dirs/2/dirs/fn.config_dir.html).

```yaml
---
longitude: 135.0
latitude: 35.0

# optional
openweathermap:
  # https://openweathermap.org/find
  city: 1850144
  # https://openweathermap.org/users/sign_up
  api_key:
    type: file
    path: ~/apikeys/openweathermap.txt

midnight:
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/midnight/*] # https://docs.rs/glob/0.3/glob/struct.Pattern.html
morning:
  # https://openweathermap.org/weather-conditions
  - on: [Thunderstorm, Dizzle, Rain] # integer (ID) or string (Main)
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/rain/*]
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/any/*]
early_afternoon:
  - on: [Thunderstorm, Dizzle, Rain]
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/rain/*]
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/any/*]
late_afternoon: # [sunset - 90min, sunset)
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/*]
evening:
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/evening/*]
```

And run `sky-color-wallpaper`(`.exe`) at the startup.

## License

Licensed under <code>[MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](http://www.apache.org/licenses/LICENSE-2.0)</code>.
