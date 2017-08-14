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

extern crate iron;
#[macro_use] extern crate hyper;
extern crate hyper_native_tls;
extern crate mount;
extern crate router;
extern crate urlencoded;
extern crate persistent;
extern crate plugin;
extern crate staticfile;
extern crate cookie;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate regex;
extern crate url;
extern crate chrono;
extern crate treexml;
extern crate fs2;
extern crate crypto;
extern crate zip;
extern crate params;
extern crate multipart;
extern crate semver;
extern crate mustache;
extern crate lazysort;
extern crate bcrypt;
extern crate uuid;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;
extern crate cldap;
extern crate rand;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate clap;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate lettre;

#[macro_use] pub mod utils;
pub mod web;

use diesel::migrations;
use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;
use simplelog::{TermLogger, SimpleLogger, WriteLogger, CombinedLogger, LogLevelFilter, SharedLogger, Config as LogConfig};

use web::server;
use web::backend::Storage;
use web::backend::db::User;

use utils::CONFIG;

use std::cmp;
use std::fs::File;
use std::io;
use std::path::PathBuf;

#[allow(dead_code)]
fn main() {

    let verbosity = match CONFIG.log.verbosity {
        0 => LogLevelFilter::Error,
        1 => LogLevelFilter::Warn,
        2 => LogLevelFilter::Info,
        3 => LogLevelFilter::Debug,
        4 | _ => LogLevelFilter::Trace,
    };
    let log_conf = LogConfig::default();

    let mut logger: Vec<Box<SharedLogger>> = vec![];
    if !CONFIG.log.quiet {
        logger.push(match TermLogger::new(verbosity, log_conf) {
            Some(termlogger) => termlogger,
            None => SimpleLogger::new(verbosity, log_conf),
        });
    }
    match CONFIG.log.logfile {
        Some(ref path) => logger.push(WriteLogger::new(cmp::max(verbosity, LogLevelFilter::Info), log_conf, File::create(path).unwrap())),
        None => {},
    }
    CombinedLogger::init(logger).unwrap();

    let config = r2d2::Config::default();
    let manager = ConnectionManager::<PgConnection>::new(CONFIG.backend.db_url.clone());
    let pool = r2d2::Pool::new(config, manager).expect("Failed to create pool.");

    //run migrations
    migrations::run_pending_migrations_in_directory(&*pool.get().unwrap(), &*PathBuf::from(&*CONFIG.backend.migrations), &mut io::stdout()).unwrap();

    {
        let connection = pool.get().unwrap();
        User::ensure_admin(&*connection, CONFIG.auth.superuser_password.clone()).unwrap();
    }

    let _iron = server::start(pool, Storage::new(PathBuf::from(&*CONFIG.backend.storage)));

    // TODO server console if we want

    // end of scope joins server thread
}
