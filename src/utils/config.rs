// Copyright (C) 2016  Max Planck Institute for Human Development
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use toml;
use clap::*;
use rand::{self, Rng};

use std::cmp;
use std::io::Read;
use std::fs::File;
use std::path::Path;

lazy_static! {
    pub static ref CONFIG: Config = {

        //TODO all via cmd

        let cmd_config = App::new("hazel")
                    .version(crate_version!())
                    .author("Victor Brekenfeld <brekenfeld@mpib-berlin.mpg.de>")
                    .about("Chocolatey-compatible Package Server")
                    .arg(Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .help("Config file location (e.g. config.toml)")
                    )
                    .arg(Arg::with_name("dburl")
                        .short("d")
                        .long("db_url")
                        .takes_value(true)
                        .help("Sets the postgres database url (e.g. postgres://postgres:postgr3s@localhost/hazel_feed_v1)")
                    )
                    .arg(Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .takes_value(true)
                        .help("HTTP port to listen on (default 80)")
                    )
                    .arg(Arg::with_name("storage")
                        .short("s")
                        .long("storage")
                        .takes_value(true)
                        .help("Sets the storage path (e.g. /var/hazel/storage/) (default current working directory)")
                    )
                    .arg(Arg::with_name("verbose")
                        .short("v")
                        .multiple(true)
                        .help("Sets the level of verbosity (may be used up to 4 times)")
                    )
                    .arg(Arg::with_name("logfile")
                        .short("l")
                        .long("logfile")
                        .takes_value(true)
                        .help("Sets a log file path (default: None)")
                    )
                    .arg(Arg::with_name("quiet")
                         .short("q")
                         .long("quiet")
                         .help("Disable console output. Hazel will not make any attempts to open stdout/err")
                     ).get_matches();

        let config_file = cmd_config.value_of("config").unwrap_or("hazel.toml");

        let db_url = cmd_config.value_of("dburl");
        let storage_path = cmd_config.value_of("storage");
        let port = value_t!(cmd_config, "port", u16);
        let logfile = cmd_config.value_of("logfile");
        let quiet = cmd_config.is_present("quiet");
        let verbosity = cmd_config.occurrences_of("verbose");

        let mut file = File::open(Path::new(config_file)).expect("Could not read config file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("IO Error reading config file");

        let mut file_config: Config = toml::from_str(&contents).expect("Config file is no valid toml");

        if db_url.is_some() {
            file_config.backend.db_url = db_url.unwrap().into();
        }
        if storage_path.is_some() {
            file_config.backend.storage = storage_path.unwrap().into();
        }
        if port.is_ok() {
            file_config.server.port = port.unwrap();
        }
        file_config.log.logfile = logfile.map(|x| { String::from(x) });
        if quiet {
            file_config.log.quiet = true;
        }
        file_config.log.verbosity = cmp::max(file_config.log.verbosity, verbosity as u8); //TODO maybe better: min(u8_max, verbosity)??

        if !file_config.log.quiet {
            println!("Using config:\n{}", toml::to_string_pretty(&file_config).unwrap());
        }

        file_config
    };
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub backend: BackendConfig,
    pub server: ServerConfig,
    pub web: WebConfig,
    pub auth: AuthenticationConfig,
    pub log: LogConfig,
}


#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    pub max_upload_filesize_mb: u32,
    pub resources: String,
}

impl Default for WebConfig
{
    fn default() -> Self {
        WebConfig {
            max_upload_filesize_mb: 10,
            resources: String::from("./resources"),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct BackendConfig {
    pub db_url: String,
    pub storage: String,
    pub migrations: String,
}

impl Default for BackendConfig
{
    fn default() -> Self {
        BackendConfig {
            db_url: String::from("postgres://localhost/hazel"),
            storage: String::from("."),
            migrations: String::from("./migrations"),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub https: Option<HTTPSConfig>,
}

impl Default for ServerConfig
{
    fn default() -> Self {
        ServerConfig {
            port: 8080,
            https: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HTTPSConfig
{
    pub certificate: String,
    pub key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    pub logfile: Option<String>,
    pub quiet: bool,
    pub verbosity: u8,
}

impl Default for LogConfig
{
    fn default() -> Self {
        LogConfig {
            logfile: None,
            quiet: false,
            verbosity: 1,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AuthenticationConfig {
    pub ldap: Option<LDAPConfig>,
    pub superuser_password: String,
    pub cookie_key: String,
    pub open_for_registration: bool,
    pub mail: Option<MailConfig>,
}

impl Default for AuthenticationConfig
{
    fn default() -> Self {
        AuthenticationConfig {
            ldap: None,
            superuser_password: String::from("admin"),
            cookie_key: {
                fn rand_string(n: u16) -> String {
                    let mut rng = rand::thread_rng();
                    (0..n).map(|_| (0x20u8 + (rng.gen::<f64>() * 96.0) as u8) as char).collect()
                }
                rand_string(64)
            },
            open_for_registration: true,
            mail: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MailConfig {
    pub hostname: String,
    pub port: Option<u16>,
    pub hello_name: String,
    pub mail_address: String,
    pub username: Option<String>,
    pub password: String,
    pub utf8: bool,
    pub encrypt: Option<bool>,
    pub authentication: Option<String>, //valid is CramMd5 or Plain
    pub fullname_website: String,
    pub domain_website: String,
}

#[derive(Serialize, Deserialize)]
pub struct LDAPConfig {
    pub server_uri: String,
    pub login_mask: String,
    pub login_mask_cn_substitution: String,
    pub common_name: String,
    pub password: String,
    pub scope: String,
    pub filter: String,
    pub filter_username_substitution: String,
    pub fullname_attr: String,
}
