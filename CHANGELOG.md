# Changelog

## [Unreleased]

### Added

- Added support for wayland compositors.
- Enabled running for non-popular Linux DE such as LeftWM.

### Changed

- Changed the default location of `sky_color_wallpaper.yml` on macOS.

    <https://github.com/xdg-rs/dirs/blob/master/directories/CHANGELOG.md#200---2020-10-22>

- [Colorizes the help message](https://docs.rs/clap/latest/clap/enum.AppSettings.html#variant.ColoredHelp) by default.

### Fixed

- Fixed the build.

## [0.3.1] - 2019-11-17Z

- Replaced [`pretty_env_logger`](https://crates.io/crates/pretty_env_logger) with [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber).

## [0.3.0] - 2019-11-05Z

- Now uses "clear sky" to choose files when it fails to get weather.

## [0.2.1] - 2019-10-12Z

### Fixed

- Fixed serialization of `Main` (`Atmosphere` → `Mist,`.., `Tornado`)

## [0.2.0] - 2019-10-04Z

### Added

- Now uses [OpenWeatherMap](https://openweathermap.org) to get weather information.

### Changed

- Modified the format of `sky_color_wallpaper.yml`.
