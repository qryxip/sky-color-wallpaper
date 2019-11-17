# sky-color-wallpaper

[![CI](https://github.com/qryxip/sky-color-wallpaper/workflows/CI/badge.svg)](https://github.com/qryxip/sky-color-wallpaper/actions?workflow=CI)
[![codecov](https://codecov.io/gh/qryxip/sky-color-wallpaper/branch/master/graph/badge.svg)](https://codecov.io/gh/qryxip/sky-color-wallpaper/branch/master)
[![dependency status](https://deps.rs/repo/github/qryxip/sky-color-wallpaper/status.svg)](https://deps.rs/repo/github/qryxip/sky-color-wallpaper)
[![Crates.io](https://img.shields.io/crates/v/sky-color-wallpaper)](https://crates.io/crates/sky-color-wallpaper)
[![Crates.io](https://img.shields.io/crates/l/sky-color-wallpaper)](https://crates.io/crates/sky-color-wallpaper)

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
longitude: 139.759
latitude: 35.6828

# optional
openweathermap:
  default: Clear
  # https://openweathermap.org/users/sign_up
  api_key:
    type: file
    path: ~/apikeys/openweathermap.txt

_:
  # https://openweathermap.org/weather-conditions
  # integer (ID) or string (Main)
  clouds: &clouds
    - Mist
    - Smoke
    - Haze
    - Dust
    - Fog
    - Sand
    - Ash
    - Clouds
  rain: &rain
    - Thunderstorm
    - Dizzle
    - Rain
    - Squall
    - Tornado
  snow: &snow
    - Snow
  clear: &clear
    - Clear

midnight:
  - patterns: [~/Pictures/wallpapers/sky_color_wallpaper/midnight/*] # https://docs.rs/glob/0.3/glob/struct.Pattern.html
morning:
  - on: *clouds
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/clouds/*]
  - on: *rain
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/rain/*]
  - on: *snow
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/snow/*]
  - on: *clear
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/morning/clear/*]
early_afternoon:
  - on: *clouds
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/clouds/*]
  - on: *rain
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/rain/*]
  - on: *snow
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/snow/*]
  - on: *clear
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/early_afternoon/clear/*]
late_afternoon: # [sunset - 90min, sunset)
  - on: *clouds
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/clouds/*]
  - on: *rain
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/rain/*]
  - on: *snow
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/snow/*]
  - on: *clear
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/late_afternoon/clear/*]
evening:
  - on: *clouds
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/evening/clouds/*]
  - on: *rain
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/evening/rain/*]
  - on: *snow
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/evening/snow/*]
  - on: *clear
    patterns: [~/Pictures/wallpapers/sky_color_wallpaper/evening/clear/*]
```

And run `sky-color-wallpaper`(`.exe`) at the startup.

## License

Licensed under <code>[MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](http://www.apache.org/licenses/LICENSE-2.0)</code>.
