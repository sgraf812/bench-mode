#[macro_use] extern crate lazy_static;
extern crate regex;
use std::process::*;

enum PowerSource {
    AC,
    DC
}

static SCHEME_GUID: &str = "0ec54905-d1ac-43db-a6df-65cbe1a1dccf";

fn powercfg(args: &[&str]) -> Output {
    Command::new("powercfg")
        .args(args)
        .output()
        .expect(&format!("Failed to execute powercfg with args {:?}", args))
}

fn unalias(alias_or_guid: &str) -> Option<String> {
    let output = powercfg(&["-query", alias_or_guid]);
    if !output.status.success() {
        return None
    }
    let haystack = &String::from_utf8_lossy(&output.stdout);
    use regex::Regex;
    lazy_static!{
        static ref GUID_REGEX: Regex = Regex::new(r"(\w{8}-\w{4}-\w{4}-\w{4}-\w{12})").unwrap();
    }
    GUID_REGEX.find(haystack).map(|m| { m.as_str().to_string() })
}

#[derive(Debug, PartialEq, Eq)]
struct PowerScheme {
    guid: String
}

enum KnownPowerSchemes {
    Active,
    Balanced,
    Min, // as in minimal energy saving
    Max,
}

impl KnownPowerSchemes {
    fn alias(&self) -> &'static str {
        match *self {
            KnownPowerSchemes::Active => "scheme_current",
            KnownPowerSchemes::Balanced => "scheme_balanced",
            KnownPowerSchemes::Min => "scheme_min",
            KnownPowerSchemes::Max => "scheme_max",
        }
    }
}

impl PowerScheme {
    fn get(alias_or_guid: &str) -> Option<PowerScheme> {
        unalias(alias_or_guid).map(|guid| { PowerScheme { guid }})
    }

    fn duplicate(&self, guid: &str) -> Option<PowerScheme> {
        if powercfg(&["-duplicatescheme", &self.guid, guid]).status.success() {
            Some(PowerScheme { guid: guid.to_string() })
        } else {
            None
        }
    }

    fn change_name(&self, name: &str, description: &str) -> Option<()> {
        if powercfg(&["-changename", &self.guid, name, description]).status.success() {
            Some(())
        } else {
            None
        }
    }

    fn set_value_index(&self, src: PowerSource, sub_guid: &str, setting: &str, idx: u32) -> Option<()> {
        let cmd = match src {
            PowerSource::AC => "setacvalueindex",
            PowerSource::DC => "setdcvalueindex",
        };
        if powercfg(&[cmd, &self.guid, sub_guid, setting, &idx.to_string()]).status.success() {
            Some(())
        } else {
            None
        }
    }

    fn activate(&self) -> Option<()> {
        if powercfg(&["-setactive", &self.guid]).status.success() {
            Some(())
        } else {
            None
        }
    }
}

fn main() {
    let active = PowerScheme::get(KnownPowerSchemes::Active.alias())
        .expect("No active power scheme");
    let benchmark = PowerScheme::get(SCHEME_GUID).unwrap_or_else(|| {
        let highest = PowerScheme::get(KnownPowerSchemes::Min.alias())
            .expect("No highest power scheme");
        let ret = highest.duplicate(SCHEME_GUID)
            .expect("Failed to clone highest power scheme");
        ret.change_name("Benchmarks", "Power scheme added by the bench-mode tool. Disables performance boost.")
            .expect("Failed to set description of duplicated power scheme");
        ret
    });

    benchmark.set_value_index(PowerSource::AC, "sub_processor", "PERFBOOSTMODE", 0)
        .expect("Couldn't set AC perf boost mode");
    benchmark.set_value_index(PowerSource::DC, "sub_processor", "PERFBOOSTMODE", 0)
        .expect("Couldn't set DC perf boost mode");
    benchmark.set_value_index(PowerSource::AC, "sub_processor", "PROCTHROTTLEMIN", 99)
        .expect("Couldn't set AC proc throttle min");
    benchmark.set_value_index(PowerSource::DC, "sub_processor", "PROCTHROTTLEMIN", 99)
        .expect("Couldn't set DC proc throttle min");
    benchmark.set_value_index(PowerSource::AC, "sub_processor", "PROCTHROTTLEMAX", 99)
        .expect("Couldn't set AC proc throttle max");
    benchmark.set_value_index(PowerSource::DC, "sub_processor", "PROCTHROTTLEMAX", 99)
        .expect("Couldn't set DC proc throttle max");

    benchmark.activate()
        .expect("Couldn't activate benchmark power scheme");
    std::thread::sleep(std::time::Duration::from_secs(10));
    active.activate()
        .expect("Couldn't activate prior active power scheme");
}
