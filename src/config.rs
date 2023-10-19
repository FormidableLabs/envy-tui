use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Mappings {
    pub navigate_up: Vec<String>,
    pub navigate_down: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigFile {
    pub mappings: Mappings,
}

pub fn load_config_file() -> Result<ConfigFile, Box<dyn Error>> {
    let file_contents =
        fs::read_to_string("/Users/endrevegh/Repos/personal/cli-network-viewer/src/config.yml")?;

    let config_file: ConfigFile = serde_yaml::from_str(file_contents.as_str())?;

    Ok(config_file)
}
