use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs::File, io::Read};
#[derive(Serialize, Debug, Deserialize)]
pub struct Proxy {
    pub addrs: Vec<Instance>,
}
#[derive(Serialize, Debug, Deserialize)]
pub struct Instance {
    host: String,
    port: u16,
    name: String,
    tags: Vec<String>,
}
pub fn read_config(path: String) -> Result<Proxy, Box<dyn Error>> {
    let mut file: File = File::open(path)?;
    let mut str = String::new();
    file.read_to_string(&mut str)?;
    let config: Proxy = toml::from_str(str.as_str())?;
    Ok(config)
}
pub fn config_to_hashmap(proxy: &Proxy) -> HashMap<String, String> {
    let mut addrs: HashMap<String, String> = HashMap::new();
    for addr in &proxy.addrs {
        addrs.insert(
            addr.name.clone(),
            format!("{}:{}", addr.host, addr.port.to_string().as_str()),
        );
    }
    addrs
}
