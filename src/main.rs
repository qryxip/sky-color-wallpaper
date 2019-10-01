#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
compile_error!("unsupported platform");

use chrono::{Local, TimeZone as _};
use env_logger_0_6::fmt::WriteStyle;
use failure::{Fallible, ResultExt as _};
use geodate::sun_transit;
use log::{debug, error, info, warn, LevelFilter};
use once_cell::sync::Lazy;
use rand::seq::SliceRandom as _;
use serde::{Deserialize as _, Deserializer};
use structopt::clap::Arg;
use structopt::StructOpt;

use std::convert::Infallible;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::File;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::str::FromStr;
use std::{env, io};

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

#[derive(serde::Deserialize, Debug)]
struct Config {
    #[serde(deserialize_with = "deser_longitude")]
    longitude: f64,
    #[serde(deserialize_with = "deser_latitude")]
    latitude: f64,
    #[serde(deserialize_with = "deser_str_seq_parsing")]
    midnight: Vec<glob::Pattern>,
    #[serde(deserialize_with = "deser_str_seq_parsing")]
    morning: Vec<glob::Pattern>,
    #[serde(deserialize_with = "deser_str_seq_parsing")]
    early_afternoon: Vec<glob::Pattern>,
    #[serde(deserialize_with = "deser_str_seq_parsing")]
    late_afternoon: Vec<glob::Pattern>,
    #[serde(deserialize_with = "deser_str_seq_parsing")]
    evening: Vec<glob::Pattern>,
}

fn deser_longitude<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let val = f64::deserialize(deserializer)?;
    if val.is_normal() && -180.0 <= val && val <= 180.0 {
        Ok(val)
    } else {
        Err(serde::de::Error::custom("expected [-180, 180]"))
    }
}

fn deser_latitude<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let val = f64::deserialize(deserializer)?;
    if val.is_normal() && -90.0 <= val && val <= 90.0 {
        Ok(val)
    } else {
        Err(serde::de::Error::custom("expected [-90, 90]"))
    }
}

fn deser_str_seq_parsing<
    'de,
    D: Deserializer<'de>,
    I: FromIterator<T>,
    T: FromStr<Err = E>,
    E: Display,
>(
    deserializer: D,
) -> Result<I, D::Error> {
    Vec::<String>::deserialize(deserializer)?
        .iter()
        .map(|s| s.parse().map_err(serde::de::Error::custom))
        .collect()
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

        let patterns = if sunrise <= now && now < midday {
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
        .map(|pattern| {
            let as_path = Path::new(pattern.as_str());
            if as_path.iter().next() == Some(OsStr::new("~")) {
                let mut acc =
                    dirs::home_dir().ok_or_else(|| failure::err_msg("Home directory not found"))?;
                as_path.iter().skip(1).for_each(|c| acc.push(c));
                acc.to_str()
                    .ok_or_else(|| failure::err_msg("The home directory is not valid UTF-8"))?
                    .parse()
                    .map_err(Into::into)
            } else if as_path
                .iter()
                .next()
                .map_or(false, |s| s.to_string_lossy().starts_with('~'))
            {
                Err(failure::err_msg(format!(
                    "Unsupported use of '~': {}",
                    as_path.display()
                )))
            } else {
                Ok(pattern.clone())
            }
        })
        .collect::<Fallible<Vec<_>>>()?;

        let paths = patterns
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
        paths
            .choose(&mut rand::thread_rng())
            .map(Clone::clone)
            .ok_or_else(|| failure::err_msg("No matches found"))
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
