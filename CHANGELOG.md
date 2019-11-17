# Changelog

## [Unreleased]

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
