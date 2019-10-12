#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
compile_error!("unsupported platform");

use chrono::{Local, TimeZone as _};
use env_logger_0_6::fmt::WriteStyle;
use failure::{Fallible, ResultExt as _};
use geodate::sun_transit;
use log::{debug, error, info, warn, LevelFilter};
use once_cell::sync::Lazy;
use rand::seq::SliceRandom as _;
use regex::Regex;
use serde::Deserialize;
use structopt::clap::Arg;
use structopt::StructOpt;

use std::convert::Infallible;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::{convert, env, io};

fn main() {
    let opt = Opt::from_args();
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("sky_color_wallpaper"), LevelFilter::Debug)
        .write_style(opt.color)
        .init();
    if let Err(err) = opt.run() {
        let msg = err.to_string();
        for line in msg.lines() {
            error!("{}", line);
        }
        if msg.ends_with('\n') {
            error!("");
        }
        for err in err.as_fail().iter_causes() {
            let msg = err.to_string();
            for (i, line) in msg.lines().enumerate() {
                match i {
                    0 => error!("Caused by: {}", line),
                    _ => error!("           {}", line),
                }
            }
            if msg.ends_with('\n') {
                error!("");
            }
        }
        process::exit(1);
    }
}

#[derive(StructOpt)]
#[structopt(author, about)]
struct Opt {
    #[structopt(
        long,
        value_name("PATH"),
        default_config_path(),
        display_order(1),
        help("Path to the config")
    )]
    config: PathBuf,
    #[structopt(
        long,
        value_name("WHEN"),
        default_value = "auto",
        possible_values(&["always", "auto", "never"]),
        parse(try_from_str = parse_write_style),
        display_order(2),
        help("Coloring")
    )]
    color: WriteStyle,
}

trait ArgExt: Sized {
    fn default_config_path(self) -> Self;
}

impl ArgExt for Arg<'static, 'static> {
    fn default_config_path(self) -> Self {
        static VALUE: Lazy<Option<PathBuf>> =
            Lazy::new(|| dirs::config_dir().map(|d| d.join("sky_color_wallpaper.yml")));
        match VALUE.as_ref() {
            None => self.required(true),
            Some(value) => self.default_value_os(value.as_ref()).required(false),
        }
    }
}

fn parse_write_style(s: &str) -> Result<WriteStyle, Infallible> {
    match s {
        "auto" => Ok(WriteStyle::Auto),
        "always" => Ok(WriteStyle::Always),
        "never" => Ok(WriteStyle::Never),
        _ => panic!(r#"expected {{"auto", "always", "never"}}"#),
    }
}

impl Opt {
    fn run(&self) -> Fallible<()> {
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
    fn load(path: &Path) -> Fallible<Self> {
        info!("Loading {}", path.display());
        serde_yaml::from_reader(File::open(path)?)
            .with_context(|_| failure::err_msg(format!("Failed to read {}", path.display())))
            .map_err(Into::into)
    }

    fn choose(&self) -> Fallible<String> {
        let now = Local::now();
        let today_beginning = now.date().and_hms(0, 0, 0).timestamp();

        let sunrise = sun_transit::get_sunrise(today_beginning, self.longitude, self.latitude)
            .unwrap_or_else(|| unimplemented!());
        let sunrise = Local.timestamp(sunrise, 0);

        let midday = sun_transit::get_midday(today_beginning, self.longitude);
        let midday = Local.timestamp(midday, 0);

        let sunset = sun_transit::get_sunset(today_beginning, self.longitude, self.latitude)
            .unwrap_or_else(|| unimplemented!());
        let sunset = Local.timestamp(sunset, 0);

        let midnight = sun_transit::get_midnight(today_beginning, self.longitude);
        let midnight = if midnight < today_beginning {
            Local.timestamp(midnight, 0) + chrono::Duration::days(1)
        } else {
            Local.timestamp(today_beginning, 0) + chrono::Duration::days(1)
        };

        debug!("sunrise  = {}", sunrise);
        debug!("midday   = {}", midday);
        debug!("sunset   = {}", sunset);
        debug!("midnight = {}", midnight);

        let weather = self
            .openweathermap
            .as_ref()
            .map::<Fallible<_>, _>(|openweathermap| {
                let api_key = openweathermap.api_key()?;
                Ok(
                    match openweathermap::current_weather_data_by_coordinates(
                        self.longitude,
                        self.latitude,
                        &api_key,
                    ) {
                        Ok(weather) => Some(weather),
                        Err(err) => {
                            warn!("{}", err);
                            None
                        }
                    },
                )
            })
            .transpose()?
            .and_then(convert::identity);
        if let Some(weather) = &weather {
            info!("Current weather:");
            for weather in weather.weather() {
                info!("- {}", weather);
            }
        }

        if sunrise <= now && now < midday {
            info!("It is morning");
            &self.morning
        } else if midday <= now && now < sunset - chrono::Duration::minutes(90) {
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
        .find(|Patterns { on, .. }| match (on, &weather) {
            (Some(on), Some(weather)) => weather.matches(on),
            (Some(_), None) => false,
            (None, _) => true,
        })
        .and_then(|p| p.choose())
        .ok_or_else(|| failure::err_msg("No matches found"))
    }
}

#[derive(Deserialize, Debug)]
struct Openweathermap {
    api_key: OpenweathermapApiKey,
}

impl Openweathermap {
    fn api_key(&self) -> Fallible<String> {
        static API_KEY: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\A\s*([0-9a-f]{32})\s*\z").unwrap());

        let OpenweathermapApiKey::File { path } = &self.api_key;
        fs::read_to_string(path)
            .map_err(Into::into)
            .and_then(|content| {
                if let Some(caps) = API_KEY.captures(&content) {
                    Ok(caps[1].to_owned())
                } else {
                    Err(failure::err_msg(r"Expected `\A\s*[0-9a-f]{32}\s*\z`"))
                }
            })
            .with_context(|_| failure::err_msg(format!("Failed to read {}", path.display())))
            .map_err(Into::into)
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

impl Patterns {
    fn choose(&self) -> Option<String> {
        let paths = self
            .patterns
            .iter()
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
            .collect::<Vec<_>>();
        info!(
            "{} file{} matched",
            paths.len(),
            if paths.len() > 1 { "s" } else { "" },
        );
        paths.choose(&mut rand::thread_rng()).map(Clone::clone)
    }
}

fn set_wallpaper(path: &str) -> Fallible<()> {
    fn pidof(program: &str) -> io::Result<bool> {
        Command::new("/usr/bin/pidof")
            .arg(program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
    }

    info!("Setting {}", path);
    if cfg!(target_os = "linux")
        && if let Some(xdg_current_desktop) = env::var_os("XDG_CURRENT_DESKTOP") {
            [OsStr::new("i3"), OsStr::new("xmonad"), OsStr::new("bspwm")]
                .contains(&&*xdg_current_desktop)
        } else {
            pidof("i3")? || pidof("xmonad")? || pidof("bspwm")?
        }
    {
        // hack
        env::set_var("XDG_CURRENT_DESKTOP", "i3");
    }
    wallpaper::set_from_path(path)
        .map_err(|e| failure::err_msg(e.to_string()))
        .with_context(|_| format!("Failed to set {}", path))?;
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
        dirs::home_dir()
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
    use log::debug;
    use serde::{Deserialize, Deserializer};
    use strum::EnumVariantNames;
    use url_1::Url;

    use std::fmt::Display;

    pub(crate) fn current_weather_data_by_coordinates(
        lon: f64,
        lat: f64,
        api_key: &str,
    ) -> Result<CurrentWeatherData, String> {
        fn hide(s: &str, api_key: &str) -> String {
            s.replace(api_key, &api_key.replace(|_| true, "█"))
        }

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| e.to_string())?;
        let mut url = "https://api.openweathermap.org/data/2.5/weather"
            .parse::<Url>()
            .unwrap();
        url.query_pairs_mut()
            .append_pair("lon", &lon.to_string())
            .append_pair("lat", &lat.to_string())
            .append_pair("APPID", api_key);
        debug!("GET: {}", hide(url.as_ref(), api_key));
        client
            .get(url)
            .send()
            .and_then(|res| {
                debug!("{}", res.status());
                res.error_for_status()
            })
            .and_then(|mut r| r.json())
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
