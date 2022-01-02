#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
compile_error!("unsupported platform");

use anyhow::{anyhow, Context as _};
use geodate::sun_transit;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom as _;
use regex::Regex;
use serde::Deserialize;
use structopt::clap::{AppSettings, Arg};
use structopt::StructOpt;
use strum::{EnumString, EnumVariantNames, IntoStaticStr};
use time::{OffsetDateTime, Time, UtcOffset};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process;
use std::{env, io};

fn main() {
    let opt = Opt::from_args();
    FmtSubscriber::builder()
        .with_ansi(opt.color.should_enable_ansi_for_stderr())
        .with_max_level(Level::INFO)
        .with_writer(io::stderr)
        .init();
    if let Err(err) = opt.run() {
        for line in format!("Error: {:?}", err).lines() {
            error!("{}", line);
        }
        process::exit(1);
    }
}

#[derive(StructOpt)]
#[structopt(author, about, setting(AppSettings::DeriveDisplayOrder))]
struct Opt {
    #[structopt(
        long,
        value_name("PATH"),
        default_config_path(),
        help("Path to the config")
    )]
    config: PathBuf,
    #[structopt(
        long,
        value_name("WHEN"),
        default_value("auto"),
        possible_values(&["auto", "never", "always"]),
        help("Coloring")
    )]
    color: ColorChoice,
}

trait ArgExt: Sized {
    fn default_config_path(self) -> Self;
}

impl ArgExt for Arg<'static, 'static> {
    fn default_config_path(self) -> Self {
        static VALUE: Lazy<Option<PathBuf>> =
            Lazy::new(|| dirs_next::config_dir().map(|d| d.join("sky_color_wallpaper.yml")));
        match VALUE.as_ref() {
            None => self.required(true),
            Some(value) => self.default_value_os(value.as_ref()).required(false),
        }
    }
}

#[derive(Debug, EnumString, IntoStaticStr, EnumVariantNames, Clone, Copy)]
#[strum(serialize_all = "kebab_case")]
enum ColorChoice {
    Auto,
    Never,
    Always,
}

impl ColorChoice {
    fn should_enable_ansi_for_stderr(self) -> bool {
        #[cfg(not(windows))]
        fn on_auto() -> bool {
            atty::is(atty::Stream::Stderr) && env::var("TERM").ok().map_or(false, |v| v != "dumb")
        }

        #[cfg(windows)]
        fn on_auto() -> bool {
            use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            use winapi_util::HandleRef;

            use std::ops::Deref;

            let term = env::var("TERM");
            let term = term.as_ref().map(Deref::deref);
            if term == Ok("dumb") || term == Ok("cygwin") {
                false
            } else if env::var_os("MSYSTEM").is_some() && term.is_ok() {
                atty::is(atty::Stream::Stderr)
            } else {
                atty::is(atty::Stream::Stderr)
                    && winapi_util::console::mode(HandleRef::stderr())
                        .ok()
                        .map_or(false, |m| m & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0)
            }
        }

        match self {
            Self::Auto => on_auto(),
            Self::Never => false,
            Self::Always => true,
        }
    }
}

impl Opt {
    fn run(&self) -> anyhow::Result<()> {
        set_wallpaper(&Config::load(&self.config)?.choose()?)
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(deserialize_with = "de::longitude")]
    longitude: f64,
    #[serde(deserialize_with = "de::latitude")]
    latitude: f64,
    openweathermap: Option<Openweathermap>,
    midnight: Vec<Patterns>,
    morning: Vec<Patterns>,
    early_afternoon: Vec<Patterns>,
    late_afternoon: Vec<Patterns>,
    evening: Vec<Patterns>,
}

impl Config {
    fn load(path: &Path) -> anyhow::Result<Self> {
        let this = File::open(path)
            .map_err(anyhow::Error::from)
            .and_then(|file| serde_yaml::from_reader(file).map_err(Into::into))
            .with_context(|| format!("Failed to read {}", path.display()))?;
        info!("Loaded {}", path.display());
        Ok(this)
    }

    fn choose(&self) -> anyhow::Result<String> {
        fn todays_events(
            today_beginning: i64,
            lon: f64,
            lat: f64,
        ) -> anyhow::Result<(
            OffsetDateTime,
            OffsetDateTime,
            OffsetDateTime,
            OffsetDateTime,
        )> {
            fn from_unix_timestamp(timestamp: i64) -> anyhow::Result<OffsetDateTime> {
                let offset = UtcOffset::current_local_offset()
                    .with_context(|| "could not get the current UTC offset")?;
                let dt = OffsetDateTime::from_unix_timestamp(timestamp)
                    .with_context(|| format!("could not recognize {}", timestamp))?;
                Ok(dt.to_offset(offset))
            }

            let sunrise = sun_transit::get_sunrise(today_beginning, lon, lat)
                .unwrap_or_else(|| unimplemented!());
            let sunrise = from_unix_timestamp(sunrise)?;

            let midday = sun_transit::get_midday(today_beginning, lon);
            let midday = from_unix_timestamp(midday)?;

            let sunset = sun_transit::get_sunset(today_beginning, lon, lat)
                .unwrap_or_else(|| unimplemented!());
            let sunset = from_unix_timestamp(sunset)?;

            let midnight = sun_transit::get_midnight(today_beginning, lon);
            let midnight = from_unix_timestamp(if midnight < today_beginning {
                midnight
            } else {
                today_beginning
            })? + time::Duration::DAY;

            info!("sunrise  = {}", sunrise);
            info!("midday   = {}", midday);
            info!("sunset   = {}", sunset);
            info!("midnight = {}", midnight);

            Ok((sunrise, midday, sunset, midnight))
        }

        let now = OffsetDateTime::now_local().with_context(|| "could not get the current time")?;
        let today_beginning = now.replace_time(Time::MIDNIGHT).unix_timestamp();

        let events = todays_events(today_beginning, self.longitude, self.latitude)?;

        let weather = self
            .openweathermap
            .as_ref()
            .map(|o| o.weather_data(self.longitude, self.latitude))
            .transpose()?;

        let paths = self.paths(now, events, weather.as_ref());

        info!(
            "{} file{} matched",
            paths.len(),
            if paths.len() > 1 { "s" } else { "" },
        );

        paths
            .choose(&mut rand::thread_rng())
            .map(Clone::clone)
            .ok_or_else(|| anyhow!("No matches found"))
    }

    fn paths(
        &self,
        now: OffsetDateTime,
        events: (
            OffsetDateTime,
            OffsetDateTime,
            OffsetDateTime,
            OffsetDateTime,
        ),
        weather: Option<&openweathermap::CurrentWeatherData>,
    ) -> Vec<String> {
        let (sunrise, midday, sunset, midnight) = events;
        if sunrise <= now && now < midday {
            info!("It is morning");
            &self.morning
        } else if midday <= now && now < sunset - time::Duration::minutes(90) {
            info!("It is early afternoon");
            &self.early_afternoon
        } else if midday <= now && now < sunset {
            info!("It is late afternoon");
            &self.late_afternoon
        } else if sunset <= now && now < midnight {
            info!("It is evening");
            &self.evening
        } else {
            info!("It is midnight");
            &self.midnight
        }
        .iter()
        .filter(|Patterns { on, .. }| match (on, &weather) {
            (Some(on), Some(weather)) => weather.matches(on),
            (Some(_), None) => false,
            (None, _) => true,
        })
        .flat_map(|Patterns { patterns, .. }| patterns)
        .flat_map(|p| glob::glob(p.as_str()).unwrap())
        .flat_map(|entry| match entry {
            Ok(path) => {
                if path.is_file() && path.to_str().is_some() {
                    Some(OsString::from(path).into_string().unwrap())
                } else {
                    warn!("Ignoring {}", path.display());
                    None
                }
            }
            Err(err) => {
                warn!("{}", err);
                None
            }
        })
        .collect()
    }
}

#[derive(Deserialize, Debug)]
struct Openweathermap {
    api_key: OpenweathermapApiKey,
}

impl Openweathermap {
    fn weather_data(
        &self,
        lon: f64,
        lat: f64,
    ) -> anyhow::Result<openweathermap::CurrentWeatherData> {
        static API_KEY: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\A\s*([0-9a-f]{32})\s*\z").unwrap());

        let OpenweathermapApiKey::File { path } = &self.api_key;
        let api_key = fs::read_to_string(path)
            .map_err(Into::into)
            .and_then(|content| {
                if let Some(caps) = API_KEY.captures(&content) {
                    Ok(caps[1].to_owned())
                } else {
                    Err(anyhow!(r"Expected `\A\s*[0-9a-f]{{32}}\s*\z`"))
                }
            })
            .with_context(|| format!("Failed to read {}", path.display()))?;

        Ok(
            openweathermap::current_weather_data_by_coordinates(lon, lat, &api_key)
                .map(|weather| {
                    info!("Current weather:");
                    for weather in weather.weather() {
                        info!("- {}", weather);
                    }
                    weather
                })
                .unwrap_or_else(|warning| {
                    warn!("{}", warning);
                    warn!("Using \"clear sky\" (id=800)");
                    openweathermap::CurrentWeatherData::default()
                }),
        )
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
enum OpenweathermapApiKey {
    File {
        #[serde(deserialize_with = "de::path_expanding_user")]
        path: PathBuf,
    },
}

#[derive(Deserialize, Debug)]
struct Patterns {
    on: Option<Vec<openweathermap::Cond>>,
    #[serde(deserialize_with = "de::patterns_expanding_user")]
    patterns: Vec<glob::Pattern>,
}

fn set_wallpaper(path: &str) -> anyhow::Result<()> {
    info!("Setting {}", path);
    wallpaper::set_from_path(path)
        .map_err(|e| anyhow!("{}", e))
        .with_context(|| format!("Failed to set {}", path))?;
    info!("Successfully set");
    Ok(())
}

mod de {
    use serde::{Deserialize as _, Deserializer};

    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    pub(crate) fn longitude<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        let val = f64::deserialize(deserializer)?;
        if val.is_normal() && -180.0 <= val && val <= 180.0 {
            Ok(val)
        } else {
            Err(serde::de::Error::custom("expected [-180, 180]"))
        }
    }

    pub(crate) fn latitude<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        let val = f64::deserialize(deserializer)?;
        if val.is_normal() && -90.0 <= val && val <= 90.0 {
            Ok(val)
        } else {
            Err(serde::de::Error::custom("expected [-90, 90]"))
        }
    }

    pub(crate) fn path_expanding_user<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PathBuf, D::Error> {
        let s = String::deserialize(deserializer)?;
        let s = expand_user(&s).map_err(serde::de::Error::custom)?;
        Ok(OsString::from(s).into())
    }

    pub(crate) fn patterns_expanding_user<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<glob::Pattern>, D::Error> {
        Vec::<String>::deserialize(deserializer)?
            .into_iter()
            .map(|s| expand_user(&s))
            .map(|r| r?.parse::<glob::Pattern>().map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()
            .map_err(serde::de::Error::custom)
    }

    fn expand_user(path: &str) -> Result<String, String> {
        expand_user_with(path, &home_dir()?)
    }

    fn expand_user_with(path: &str, home: &str) -> Result<String, String> {
        fn fold_unwrap(head: &str, tail: &Path) -> String {
            tail.iter()
                .fold(PathBuf::from(head), |p, s| p.join(s))
                .into_os_string()
                .into_string()
                .unwrap()
        }

        if let Ok(tail) = Path::new(path).strip_prefix("~") {
            Ok(fold_unwrap(home, tail))
        } else if path.starts_with('~') {
            Err(format!("Unsupported use of '~': {:?}", path))
        } else {
            Ok(fold_unwrap("", Path::new(path)))
        }
    }

    fn home_dir() -> Result<String, String> {
        dirs_next::home_dir()
            .ok_or_else(|| "Home directory not found".to_owned())?
            .into_os_string()
            .into_string()
            .map_err(|h| format!("The home directory is not valid UTF-8: {:?}", h))
    }

    #[cfg(test)]
    mod tests {
        use std::path::{self, Path};

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        #[test]
        fn test_expand_user_with() {
            debug_assert_eq!(path::MAIN_SEPARATOR, '/');
            debug_assert!(Path::new("/").is_absolute());

            assert_eq!(
                super::expand_user_with("foo/bar", "/home/user"),
                Ok("foo/bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("/foo/bar", "/home/user"),
                Ok("/foo/bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("~/foo/bar", "/home/user"),
                Ok("/home/user/foo/bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("~user/foo", "/home/user"),
                Err(r#"Unsupported use of '~': "~user/foo""#.to_owned()),
            );
        }

        #[cfg(windows)]
        #[test]
        fn test_expand_user_with() {
            debug_assert_eq!(path::MAIN_SEPARATOR, '\\');
            debug_assert!(Path::new("\\").is_relative());

            assert_eq!(
                super::expand_user_with("foo/bar", r"C:\Users\user"),
                Ok(r"foo\bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("/foo/bar", r"C:\Users\user"),
                Ok(r"\foo\bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("~/foo/bar", r"C:\Users\user"),
                Ok(r"C:\Users\user\foo\bar".to_owned()),
            );
            assert_eq!(
                super::expand_user_with("~user/foo", r"C:\Users\user"),
                Err(r#"Unsupported use of '~': "~user/foo""#.to_owned()),
            );
        }
    }
}

mod openweathermap {
    use itertools::Itertools as _;
    use serde::{Deserialize, Deserializer};
    use strum::EnumVariantNames;
    use tracing::info;
    use url::Url;

    use std::fmt::Display;

    pub(crate) fn current_weather_data_by_coordinates(
        lon: f64,
        lat: f64,
        api_key: &str,
    ) -> Result<CurrentWeatherData, String> {
        fn hide(s: &str, api_key: &str) -> String {
            s.replace(api_key, &api_key.replace(|_| true, "█"))
        }

        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| e.to_string())?;
        let mut url = "https://api.openweathermap.org/data/2.5/weather"
            .parse::<Url>()
            .unwrap();
        url.query_pairs_mut()
            .append_pair("lon", &lon.to_string())
            .append_pair("lat", &lat.to_string())
            .append_pair("APPID", api_key);
        info!("GET: {}", hide(url.as_ref(), api_key));
        client
            .get(url)
            .send()
            .and_then(|res| {
                info!("{}", res.status());
                res.error_for_status()
            })
            .and_then(reqwest::blocking::Response::json)
            .map_err(|e| hide(&e.to_string(), api_key))
    }

    #[derive(Debug)]
    pub(crate) enum Cond {
        Id(u64),
        Main(WeatherMain),
    }

    impl<'de> Deserialize<'de> for Cond {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Repr {
                Id(u64),
                Main(WeatherMain),
                InvalidMain(String),
            }

            match Repr::deserialize(deserializer).map_err(|_| {
                static MSG: &str = "expected unsigned 64-bit integer (ID) or string (Main)";
                serde::de::Error::custom(MSG)
            })? {
                Repr::Id(id) => Ok(Self::Id(id)),
                Repr::Main(main) => Ok(Self::Main(main)),
                Repr::InvalidMain(main) => Err(serde::de::Error::custom(format!(
                    "unknown variant `{}`, expected integer or one of {}",
                    main,
                    WeatherMain::variants()
                        .iter()
                        .format_with(", ", |s, f| f(&format_args!("`{}`", s))),
                ))),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct CurrentWeatherData {
        weather: Vec<Weather>,
    }

    impl CurrentWeatherData {
        pub(crate) fn weather<'a>(&'a self) -> &[impl Display + 'a] {
            &self.weather
        }

        pub(crate) fn matches(&self, conds: &[Cond]) -> bool {
            self.weather.iter().any(|weather| {
                conds.iter().any(|cond| match cond {
                    Cond::Id(id) => weather.id == *id,
                    Cond::Main(main) => weather.main == *main,
                })
            })
        }
    }

    impl Default for CurrentWeatherData {
        fn default() -> Self {
            Self {
                weather: vec![Weather {
                    id: 800,
                    main: WeatherMain::Clear,
                    description: "clear sky (default value from sky-color-wallpaper)".to_owned(),
                }],
            }
        }
    }

    #[derive(Deserialize, Debug, derive_more::Display)]
    #[display(fmt = "{:?} (id={})", description, id)]
    struct Weather {
        id: u64,
        main: WeatherMain,
        description: String,
    }

    // https://openweathermap.org/weather-conditions
    #[derive(Deserialize, EnumVariantNames, Clone, Copy, PartialEq, Debug)]
    pub(crate) enum WeatherMain {
        Thunderstorm,
        Dizzle,
        Rain,
        Snow,
        Mist,
        Smoke,
        Haze,
        Dust,
        Fog,
        Sand,
        Ash,
        Squall,
        Tornado,
        Clear,
        Clouds,
    }
}
