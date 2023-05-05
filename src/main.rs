use std::{fmt::Display, time::Duration};

use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Timelike};
use clap::Parser;

#[derive(clap::Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command()]
    Battery {
        #[arg(long)]
        device_path: String,
        #[arg(long, default_value = "false")]
        debug: bool,
    },
    #[command()]
    Time {
        #[arg(long, default_value = "false")]
        seconds: bool,
        #[arg(long, default_value = "true")]
        date: bool,
    },
    #[command()]
    TimeZh {
        #[arg(long, default_value = "false")]
        seconds: bool,
        #[arg(long, default_value = "true")]
        date: bool,
    },
    #[command()]
    Memory,
}

#[derive(Default)]
pub struct PangoSpan {
    pub color: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<String>,
    pub weight: Option<String>,
}

impl Display for PangoSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<span ")?;
        if let Some(color) = &self.color {
            write!(f, "color=\"{}\" ", color)?;
        }
        if let Some(font_family) = &self.font_family {
            write!(f, "font_family=\"{}\" ", font_family)?;
        }
        if let Some(font_size) = &self.font_size {
            write!(f, "font_size=\"{}\" ", font_size)?;
        }
        if let Some(weight) = &self.weight {
            write!(f, "weight=\"{}\" ", weight)?;
        }
        write!(f, ">")?;
        Ok(())
    }
}

macro_rules! pango {
    ($text: expr, $($key:ident = $value:expr),* $(,)?) => {{
        let span = PangoSpan {
            $($key: Some($value.to_string()),)*
            ..PangoSpan::default()
        };
        let mut s = span.to_string();
        s.push_str(&format!("{}</span>", $text));
        s
    }};
}

pub struct Theme {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub index_16: String,
    pub index_17: String,
}

macro_rules! impl_theme_color {
    ($name: ident) => {
        pub fn $name(&self) -> &str {
            &self.$name
        }
    };
}

impl Theme {
    impl_theme_color!(foreground);
    impl_theme_color!(background);
    impl_theme_color!(black);
    impl_theme_color!(red);
    impl_theme_color!(green);
    impl_theme_color!(yellow);
    impl_theme_color!(blue);
    impl_theme_color!(magenta);
    impl_theme_color!(cyan);
    impl_theme_color!(white);
    impl_theme_color!(index_16);
    impl_theme_color!(index_17);

    pub fn tokyonight_normal() -> Self {
        Self {
            foreground: "#c0caf5".to_string(),
            background: "#1a1b26".to_string(),
            black: "#15161e".to_string(),
            red: "#f7768e".to_string(),
            green: "#9ece6a".to_string(),
            yellow: "#e0af68".to_string(),
            blue: "#7aa2f7".to_string(),
            magenta: "#bb9af7".to_string(),
            cyan: "#7dcfff".to_string(),
            white: "#a9b1d6".to_string(),
            index_16: "#ff9e64".to_string(),
            index_17: "#db4b4b".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BatteryState {
    Charging,
    Discharging,
    Full,
}

#[derive(Debug)]
struct BatteryInfo {
    energy_full: f64,
    energy: f64,
    time_to_empty_full: f64,
    state: BatteryState,
}

impl BatteryInfo {
    fn percentage(&self) -> i32 {
        (self.energy / self.energy_full * 100.0) as i32
    }

    fn time_to_empty_full(&self) -> Duration {
        Duration::from_secs_f64(self.time_to_empty_full * 3600.0)
    }

    fn time_to_empty_full_str(&self) -> String {
        let duration = self.time_to_empty_full();
        let hours = duration.as_secs() / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        format!("{:02}:{:02}", hours, minutes)
    }
}

fn get_battery_info(device_path: &str) -> Result<BatteryInfo> {
    let result = std::process::Command::new("upower")
        .arg("-i")
        .arg(device_path)
        .output()
        .context("running upower")?;
    let output = String::from_utf8(result.stdout).context("converting upower output to utf-8")?;
    let energy_full = output
        .lines()
        .find(|line| line.trim_start().starts_with("energy-full:"))
        .ok_or_else(|| anyhow!("energy-full not found"))?
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("energy-full format is invalid"))?
        .parse::<f64>()?;
    let energy = output
        .lines()
        .find(|line| line.trim_start().starts_with("energy:"))
        .ok_or_else(|| anyhow!("energy not found"))?
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("energy format is invalid"))?
        .parse::<f64>()?;
    let mut time_to_line = output
        .lines()
        .find(|line| line.trim_start().starts_with("time to"))
        .ok_or_else(|| anyhow!("`time to {{empty | full}}` not found"))?
        .split_whitespace();
    let mut time_to_empty_full = time_to_line
        .nth(3)
        .ok_or_else(|| anyhow!("`time to {{empty | full}}` format is invalid"))?
        .parse::<f64>()?;
    let time_to_empty_full_unit = time_to_line
        .nth(0)
        .ok_or_else(|| anyhow!("`time to {{empty | full}}` format is invalid"))?;
    if time_to_empty_full_unit == "minutes" {
        time_to_empty_full /= 60.0;
    }
    let state = output
        .lines()
        .find(|line| line.trim_start().starts_with("state:"))
        .ok_or_else(|| anyhow!("state not found"))?
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("state format is invalid"))
        .map(|s| match s {
            "charging" => BatteryState::Charging,
            "discharging" => BatteryState::Discharging,
            _ => BatteryState::Full,
        })?;
    Ok(BatteryInfo {
        energy_full,
        energy,
        time_to_empty_full,
        state,
    })
}

struct MemoryInfo {
    total: u64,
    used: u64,
}

impl MemoryInfo {
    pub fn total_mib(&self) -> u64 {
        self.total / 1024
    }

    pub fn used_mib(&self) -> u64 {
        self.used / 1024
    }
}

fn get_memory_info() -> Result<MemoryInfo> {
    let result = std::process::Command::new("free")
        .output()
        .context("running `free`")?;
    let output = String::from_utf8(result.stdout).context("converting `free` output to utf-8")?;
    let line = output
        .lines()
        .nth(1)
        .ok_or_else(|| anyhow!("`free` output is invalid"))?;
    let mut fields = line.split_whitespace();
    let total = fields
        .nth(1)
        .ok_or_else(|| anyhow!("`free` output is invalid"))?
        .parse::<u64>()?;
    let used = fields
        .next()
        .ok_or_else(|| anyhow!("`free` output is invalid"))?
        .parse::<u64>()?;
    Ok(MemoryInfo { total, used })
}

fn main() -> Result<()> {
    use Command::*;
    let theme = Theme::tokyonight_normal();
    let command = Cli::parse().command;
    match command {
        Battery { device_path, debug } => {
            let battery_info = if debug {
                get_battery_info(&device_path)?
            } else {
                if let Ok(battery_info) = get_battery_info(&device_path) {
                    battery_info
                } else {
                    println!("🔌");
                    return Ok(());
                }
            };
            let icon = if battery_info.state == BatteryState::Charging {
                "🔌"
            } else {
                if battery_info.percentage() >= 20 {
                    "🔋"
                } else {
                    "🪫"
                }
            };
            println!(
                "{icon} {pct}{pct_sign} {time}",
                icon = pango!(icon, font_size = "120%"),
                pct = pango!(
                    battery_info.percentage(),
                    color = theme.foreground(),
                    weight = "ultrabold",
                    font_size = "110%",
                ),
                pct_sign = pango!("%", color = theme.white()),
                time = pango!(battery_info.time_to_empty_full_str(), color = theme.white()),
            );
            Ok(())
        }
        Time { seconds, date } => {
            let time = chrono::Local::now();
            let time_str = if seconds {
                time.format("%H:%M:%S")
            } else {
                time.format("%H:%M")
            };
            let time_of_day = match time.hour() {
                0..=11 => "AM",
                12..=23 => "PM",
                _ => unreachable!(),
            };
            if date {
                println!(
                    "{date} {time} {}",
                    time_of_day,
                    date = time.format("%Y-%m-%d"),
                    time = time_str,
                );
            } else {
                println!("{} {}", time_str, time_of_day);
            }
            Ok(())
        }
        TimeZh { seconds, date } => {
            let time = chrono::Local::now();
            let time_str = if seconds {
                time.format("%H:%M:%S")
            } else {
                time.format("%H:%M")
            };
            let time_of_day = match time.hour() {
                0..=5 => "凌晨",
                6..=11 => "上午",
                12..=13 => "中午",
                14..=17 => "下午",
                18..=23 => "晚上",
                _ => unreachable!(),
            };
            if date {
                println!(
                    "{date} {time} {tod}",
                    time = pango!(
                        time_str,
                        color = theme.foreground(),
                        weight = "ultrabold",
                        font_size = "120%",
                    ),
                    tod = pango!(time_of_day, color = theme.white(),),
                    date = {
                        let y = time.year();
                        let m = time.month();
                        let d = time.day();
                        format!(
                            "{y}{nian}{m}{yue}{d}{ri}",
                            y = pango!(
                                y,
                                color = theme.foreground(),
                                font_size = "110%",
                                weight = "ultrabold"
                            ),
                            m = pango!(
                                m,
                                color = theme.foreground(),
                                font_size = "110%",
                                weight = "ultrabold"
                            ),
                            d = pango!(
                                d,
                                color = theme.foreground(),
                                font_size = "110%",
                                weight = "ultrabold"
                            ),
                            nian = pango!("年", color = theme.white()),
                            yue = pango!("月", color = theme.white()),
                            ri = pango!("日", color = theme.white()),
                        )
                    },
                );
            } else {
                println!("{} {}", time_str, time_of_day);
            }
            Ok(())
        }
        Memory => {
            let memory_info = get_memory_info()?;
            println!(
                "{used}{div}{total}{mib}",
                used = pango!(
                    memory_info.used_mib(),
                    color = theme.foreground(),
                    weight = "ultrabold",
                    font_size = "110%",
                ),
                total = pango!(
                    memory_info.total_mib(),
                    color = theme.foreground(),
                    weight = "ultrabold",
                    font_size = "110%",
                ),
                div = pango!("/", color = theme.white()),
                mib = pango!("MiB", color = theme.white()),
            );
            Ok(())
        }
    }
}
