//! Static command catalogs shipped with the binary. Loaded once per process
//! and reused for every hint query.

use std::sync::OnceLock;

use crate::Platform;

const LINUX_JSON: &str = include_str!("../data/linux.json");
const CISCO_JSON: &str = include_str!("../data/cisco_ios.json");
const JUNIPER_JSON: &str = include_str!("../data/juniper.json");
const MIKROTIK_JSON: &str = include_str!("../data/mikrotik.json");
const ARUBA_JSON: &str = include_str!("../data/aruba.json");

fn parse(json: &'static str) -> Vec<String> {
    serde_json::from_str(json).expect("bundled catalog must be valid JSON")
}

pub fn for_platform(platform: Platform) -> &'static [String] {
    match platform {
        Platform::Generic => {
            static EMPTY: Vec<String> = Vec::new();
            &EMPTY
        }
        Platform::Linux => {
            static C: OnceLock<Vec<String>> = OnceLock::new();
            C.get_or_init(|| parse(LINUX_JSON))
        }
        Platform::CiscoIos => {
            static C: OnceLock<Vec<String>> = OnceLock::new();
            C.get_or_init(|| parse(CISCO_JSON))
        }
        Platform::Juniper => {
            static C: OnceLock<Vec<String>> = OnceLock::new();
            C.get_or_init(|| parse(JUNIPER_JSON))
        }
        Platform::Mikrotik => {
            static C: OnceLock<Vec<String>> = OnceLock::new();
            C.get_or_init(|| parse(MIKROTIK_JSON))
        }
        Platform::Aruba => {
            static C: OnceLock<Vec<String>> = OnceLock::new();
            C.get_or_init(|| parse(ARUBA_JSON))
        }
    }
}

pub fn all_platforms() -> &'static [Platform] {
    &[
        Platform::Generic,
        Platform::Linux,
        Platform::CiscoIos,
        Platform::Juniper,
        Platform::Mikrotik,
        Platform::Aruba,
    ]
}
