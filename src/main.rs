use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Timelike;
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
    let time_to_empty_full = output
        .lines()
        .find(|line| line.trim_start().starts_with("time to"))
        .ok_or_else(|| anyhow!("`time to {{empty | full}}` not found"))?
        .split_whitespace()
        .nth(3)
        .ok_or_else(|| anyhow!("`time to {{empty | full}}` format is invalid"))?
        .parse::<f64>()?;
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

fn main() -> Result<()> {
    use Command::*;
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
                "{icon} {pct}% ({time} {state})",
                icon = icon,
                pct = battery_info.percentage(),
                time = battery_info.time_to_empty_full_str(),
                state = if battery_info.state == BatteryState::Charging {
                    "until full"
                } else {
                    "remaining"
                }
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
                    "{date} {} {}",
                    time_str,
                    time_of_day,
                    date = time.format("%Y年%m月%d日"),
                );
            } else {
                println!("{} {}", time_str, time_of_day);
            }
            Ok(())
        }
    }
}
