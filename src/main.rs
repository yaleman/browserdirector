use std::collections::HashMap;
use std::process::Command;

use fork::{daemon, Fork};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

static OPEN_PATH: &str = "/usr/bin/open";
static APP_CHROME: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
static APP_EDGE: &str = "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge";
static APP_FIREFOX: &str = "/Applications/Firefox.app/Contents/MacOS/firefox";

#[derive(Clone, Serialize, Debug, Deserialize, Eq, PartialEq, Default)]
enum Apps {
    Chrome,
    Edge,
    Firefox,
    #[default]
    Default,
}

impl From<Apps> for &str {
    fn from(value: Apps) -> Self {
        match value {
            Apps::Chrome => APP_CHROME,
            Apps::Edge => APP_EDGE,
            Apps::Firefox => APP_FIREFOX,
            Apps::Default => OPEN_PATH,
        }
    }
}

type ProfileHashMap = HashMap<String, String>;
#[derive(Clone, Serialize, Debug, Deserialize)]
struct Profile {
    name: String,
    browser: Option<Apps>,
    browser_profile: Option<String>,
    domains: Vec<String>,
}

impl Profile {
    fn profile_dir(&self, profiles: &ProfileHashMap) -> Option<String> {
        match &self.browser_profile {
            None => None,
            Some(profile) => profiles.get(profile).map(|p| p.to_owned()),
        }
    }
}

#[derive(Serialize, Debug, Deserialize)]
struct ConfigFile(Vec<Profile>);

fn get_profile_name(path: &std::path::Path) -> Option<String> {
    let file_contents = std::fs::read_to_string(path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&file_contents).unwrap();
    json["profile"]["name"].as_str().map(|s| s.to_string())
}

fn find_edge_profiles() -> ProfileHashMap {
    let home = dirs::home_dir().unwrap();

    find_chromium_profiles(format!(
        "{}/Library/Application Support/Microsoft Edge",
        home.display()
    ))
}

fn load_profile() -> ConfigFile {
    // let home = dirs::home_dir().unwrap();
    // let config_path = format!("{}/.config/open-url/config.json", home.display());
    let config_path = "./browserselector.json";
    let file_contents = std::fs::read_to_string(config_path).unwrap();
    let json: ConfigFile = serde_json::from_str(&file_contents).unwrap();
    json
}

fn find_chromium_profiles(path: String) -> ProfileHashMap {
    // user's home dir
    let mut results: ProfileHashMap = HashMap::new();

    for entry in WalkDir::new(path) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            let dirname = entry.path().as_os_str().to_str().unwrap();
            let filename = entry.file_name().to_str().unwrap();
            if dirname.contains("Profile") && filename == "Preferences" {
                if let Some(profile_name) = get_profile_name(entry.path()) {
                    results.insert(
                        profile_name,
                        entry
                            .path()
                            .parent()
                            .unwrap()
                            .components()
                            .last()
                            .unwrap()
                            .as_os_str()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                }
            }
        }
        // println!("{}", entry.path().display());
    }
    results
}
fn main() {
    let config = load_profile();
    let edge_profiles = find_edge_profiles();

    // take the first argument to this program as a url
    let url = std::env::args().nth(1).expect("no url given");
    // parse the url
    let url = url::Url::parse(&url).expect("invalid url");
    println!("{:?}", url.host());

    let mut args: Vec<String> = Vec::new();
    let mut app = OPEN_PATH;

    for profile in config.0 {
        if profile
            .domains
            .contains(&url.host_str().unwrap().to_string())
        {
            match profile.browser.clone().unwrap_or_default() {
                Apps::Chrome | Apps::Edge => {
                    if let Some(profile_dir) = profile.profile_dir(&edge_profiles) {
                        args.extend([format!("--profile-directory={}", profile_dir)]);
                    }
                }
                Apps::Firefox => {
                    args.push("--new-tab".to_string());
                }
                Apps::Default => {}
            }
            let browser = profile.browser.clone();
            let browser = browser.unwrap_or_default();
            app = browser.into();
        }
    }

    args.push(url.as_str().to_string());
    println!("Running {} {:?}", app, args);
    run_command(app, args);
}

fn run_command(cmd: &str, args: Vec<String>) {
    if let Ok(Fork::Child) = daemon(false, false) {
        Command::new(cmd)
            .args(&args)
            .output()
            .unwrap_or_else(|_| panic!("Failed to run {} {:?}", cmd, args));
    }
}
