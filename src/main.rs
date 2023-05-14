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
        #[arg(long, default_value = "true")]
        am_pm: bool,
    },
    #[command()]
    Memory,
    #[command()]
    SinkVolume,
    #[command()]
    Brightness,
    #[command()]
    VirshActive,
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

pub mod pulseaudio {
    use anyhow::{anyhow, Result};

    pub struct Volume {
        left: u64,
        right: u64,
        mute: bool,
    }

    impl Volume {
        pub fn left_pct(&self) -> u64 {
            self.left * 100 / 65530 // not std::u16::MAX for some reason
        }

        pub fn right_pct(&self) -> u64 {
            self.right * 100 / 65530 // not std::u16::MAX for some reason
        }

        fn icon(value: u64, mute: bool) -> &'static str {
            if mute {
                return "🔇";
            }
            match value {
                0 => "🔇",
                1..=33 => "🔈",
                34..=66 => "🔉",
                _ => "🔊",
            }
        }

        pub fn left_icon(&self) -> &'static str {
            Self::icon(self.left_pct(), self.mute)
        }

        pub fn right_icon(&self) -> &'static str {
            Self::icon(self.right_pct(), self.mute)
        }
    }

    pub fn volume() -> Result<Volume> {
        let result = std::process::Command::new("pactl")
            .arg("get-sink-volume")
            .arg("@DEFAULT_SINK@")
            .output()?;
        let output = String::from_utf8(result.stdout)?;
        let line = output
            .lines()
            .next()
            .ok_or_else(|| anyhow!("`pactl` output is invalid"))?;
        let mut fields = line.split_whitespace();
        // Volume: front-left: 65530 / 100% / -0.00 dB,   front-right: 65530 / 100% / -0.00 dB
        let left = fields
            .nth(2)
            .ok_or_else(|| anyhow!("`pactl` output is invalid"))?
            .parse::<u64>()?;
        let right = fields
            .nth(6)
            .ok_or_else(|| anyhow!("`pactl` output is invalid"))?
            .parse::<u64>()?;
        let result = std::process::Command::new("pactl")
            .arg("get-sink-mute")
            .arg("@DEFAULT_SINK@")
            .output()?;
        let output = String::from_utf8(result.stdout)?;
        let line = output
            .lines()
            .next()
            .ok_or_else(|| anyhow!("`pactl` output is invalid"))?;
        let mute = line.contains("yes");
        Ok(Volume { left, right, mute })
    }
}

pub mod brightness {
    use anyhow::Result;

    pub struct BrightnessInfo {
        pub current: u64,
        pub max: u64,
    }

    impl BrightnessInfo {
        pub fn pct(&self) -> u64 {
            self.current * 100 / self.max
        }

        pub fn icon(&self) -> &'static str {
            match self.pct() {
                0 => "🌑",
                1..=33 => "🌒",
                34..=66 => "🌓",
                67..=99 => "🌔",
                _ => "🌕",
            }
        }
    }

    pub fn info() -> Result<BrightnessInfo> {
        let result = std::process::Command::new("brightnessctl")
            .arg("info")
            .output()?;
        let output = String::from_utf8(result.stdout).unwrap();
        // Default max 1 to avoid div by 0
        let mut out = BrightnessInfo { current: 0, max: 1 };
        for line in output.lines() {
            if line.trim_start().starts_with("Current brightness:") {
                out.current = line
                    .split_whitespace()
                    .nth(2)
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
            } else if line.trim_start().starts_with("Max brightness:") {
                out.max = line
                    .split_whitespace()
                    .nth(2)
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
            }
        }
        Ok(out)
    }
}

pub mod virsh {
    use anyhow::{anyhow, Result};
    /// Represents the state returned by the virsh list command
    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct State {
        /// the active vms
        active: Vec<String>,
        /// the inactive vms
        inactive: Vec<String>,
    }

    pub fn list() -> Result<State> {
        let result = std::process::Command::new("virsh")
            .arg("list")
            .arg("--all")
            .output()?;
        let output = String::from_utf8(result.stdout).unwrap();
        let mut active = Vec::new();
        let mut inactive = Vec::new();
        for line in output.trim().lines().skip(2) {
            let mut fields = line.trim_start().split_whitespace();
            // skip the id
            fields.next();
            // FIXME: assume that the name has no spaces
            let name = fields
                .next()
                .ok_or_else(|| anyhow!("invalid format"))?
                .to_string();
            let state = fields.next().ok_or_else(|| anyhow!("invalid format"))?;
            match state {
                "running" => active.push(name),
                "shut" => inactive.push(name),
                _ => {}
            }
        }
        Ok(State { active, inactive })
    }
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
        TimeZh {
            seconds,
            date,
            am_pm,
        } => {
            let time = chrono::Local::now();
            let time_str = {
                let mut h = time.hour() % if am_pm { 12 } else { 24 };
                // 12-hour clock 0:00 => 12:00, but in 24 hour clock 0:00 => 0:00
                if h == 0 && am_pm {
                    h = 12;
                }
                let m = time.minute();
                let s = time.second();
                if seconds {
                    format!("{:02}:{:02}:{:02}", h, m, s)
                } else {
                    format!("{:02}:{:02}", h, m)
                }
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
        SinkVolume => {
            let volume_info = pulseaudio::volume()?;
            println!(
                "{icon} {left}{pct}",
                icon = pango!(volume_info.left_icon(), font_size = "120%"),
                left = pango!(
                    volume_info.left_pct(),
                    color = theme.foreground(),
                    weight = "ultrabold",
                    font_size = "110%",
                ),
                pct = pango!("%", color = theme.white()),
            );
            Ok(())
        }
        Brightness => {
            let brightness_info = brightness::info()?;
            println!(
                "{icon} {value}{pct}",
                icon = pango!(brightness_info.icon(), font_size = "120%"),
                value = pango!(
                    brightness_info.pct(),
                    color = theme.foreground(),
                    weight = "ultrabold",
                    font_size = "110%",
                ),
                pct = pango!("%", color = theme.white()),
            );
            Ok(())
        }
        VirshActive => {
            let state = virsh::list()?;
            print!("{state:?}");
            Ok(())
        }
    }
}
