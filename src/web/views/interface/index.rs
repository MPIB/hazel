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

use iron::{Request, Response, IronResult};
use iron::status;
use iron::mime::Mime;
use persistent::Read;
use router::Router;
use mustache::{Template, compile_path};
use lazysort::SortedBy;

use std::path::PathBuf;

use ::web::server::ConnectionPoolKey;
use ::web::backend::db::{Package as Pkg, PackageVersion, User};
use ::utils::middleware::Authenticated;
use ::utils::CONFIG;

lazy_static! {
    static ref TEMPLATE: Template = compile_path(PathBuf::from(CONFIG.web.resources.clone()).join("index.html")).unwrap();
}

#[derive(Serialize)]
struct Index {
    repo: Vec<Package>,
    pages: Vec<Page>,
    loggedin: bool,
    username: Option<String>,
    confirmation_required: bool,
    open_for_registration: bool,
    confirmed: bool,
    api: Option<API>,
}

#[derive(Serialize)]
struct API
{
    key: String,
    maxfilesize: u32,
}

#[derive(Serialize)]
struct Package {
    id: String,
    version: String,
    icon: String,
    title: String,
    description: String,
}

#[derive(Serialize)]
struct Page {
    active: String,
    number: usize,
}

impl From<PackageVersion> for Package {
    fn from(pkgver: PackageVersion) -> Package {
        Package {
            id: pkgver.id().to_owned(),
            version: format!("{}", pkgver.version()),
            icon: pkgver.icon_url.unwrap_or(String::new()),
            title: pkgver.title.unwrap_or(String::new()),
            description: pkgver.summary.unwrap_or(String::new()),
        }
    }
}

pub fn index(req: &mut Request) -> IronResult<Response> {
    let ref page: usize = req.extensions.get::<Router>().unwrap().find("page").unwrap_or("1").parse().unwrap_or(1);

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let mut index = match req.extensions.get::<Authenticated>().unwrap() {
        &(true, Some(ref username)) => {
            match User::get(&*connection, username) {
                Ok(user) => Index { repo: Vec::new(), pages: Vec::new(), loggedin: true, username: Some(user.name.clone()), open_for_registration: CONFIG.auth.open_for_registration,
                    confirmation_required: CONFIG.auth.mail.is_some(), confirmed: user.confirmed(), api: match user.apikey() {
                        Some(key) => Some(API { key: key, maxfilesize: CONFIG.web.max_upload_filesize_mb }),
                        None => None,
                }},
                Err(_) => return Ok(Response::with((status::Unauthorized, "User does not exist anymore"))),
            }
        },
        _ => Index { repo: Vec::new(), pages: Vec::new(), loggedin: false, username: None, api: None, confirmed: false, confirmation_required: CONFIG.auth.mail.is_some(), open_for_registration: CONFIG.auth.open_for_registration },
    };

    //TODO limit packages when we reach a high count and add helper for quicker updated retrieval
    let packages = match Pkg::all(&*connection) {
        Ok(packages) => {
            let mut versions = Vec::new();
            for pkg in packages {
                match pkg.newest_version(&*connection) {
                    Ok(version) => versions.push(version),
                    Err(_) => {
                        //package has no versions
                        warn!("Package without version (garbage) found in db. (id: {})", pkg.id());
                    },
                }
            };
            versions
        },
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let page_count = packages.len() / 50 + if packages.len() % 50 > 0 { 1 } else { 0 };
    for i in 1..page_count+1 {
        index.pages.push(Page {
            active: if &i == page { String::from("class=active") } else { String::new() },
            number: i,
        });
    }

    for pkg in packages.into_iter().sorted_by(|a, b| a.id().cmp(&b.id())).skip(10*(page-1)).take(50) {
        index.repo.push(Package::from(pkg));
    }

    let mut buffer = Vec::new();
    TEMPLATE.render(&mut buffer, &index).unwrap(); //TODO
    Ok(Response::with((status::Ok, buffer, {
        let mime: Mime = "text/html".parse().unwrap();
        mime
    })))
}
