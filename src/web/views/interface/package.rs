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
use iron::modifiers::Redirect;
use persistent::Read;
use router::Router;
use ::utils::error::BackendError;
use mustache::{Template, compile_path};
use lazysort::SortedBy;

use std::path::PathBuf;

use ::web::server::ConnectionPoolKey;
use ::utils::CONFIG;
use ::utils::middleware::Authenticated;
use ::web::backend::db::{User, Package, PackageVersion};

lazy_static! {
    static ref TEMPLATE: Template = compile_path(PathBuf::from(CONFIG.web.resources.clone()).join("package.html")).unwrap();
}

#[derive(Serialize, Debug)]
struct Version
{
    active: bool,
    version: String,
    creation_date: String,
    title: Option<String>,
    summary: Option<String>,
    updated: String,
    description: Option<String>,
    version_download_count: i64,
    release_notes: Option<String>,
    hash: Option<String>,
    hash_algorithm: Option<String>,
    size: i64,
    icon_url: Option<String>,
}

impl From<PackageVersion> for Version {
    fn from(pkgver: PackageVersion) -> Version {
        Version {
            active: false,
            version: format!("{}", pkgver.version()),
            creation_date: format!("{}", pkgver.creation_date()),
            title: pkgver.title.clone(),
            summary: pkgver.summary.clone().map(|x| String::from(x.trim())),
            updated: format!("{}", pkgver.last_updated()),
            description: pkgver.description.clone().map(|x| String::from(x.trim())),
            version_download_count: pkgver.version_download_count(),
            release_notes: pkgver.release_notes.clone().map(|x| String::from(x.trim())),
            hash: pkgver.hash().cloned(),
            hash_algorithm: pkgver.hash_algorithm().cloned(),
            size: pkgver.byte_size(),
            icon_url: pkgver.icon_url.clone(),
        }
    }
}

#[derive(Serialize)]
struct API
{
    key: String,
}

#[derive(Serialize)]
struct PackagePage
{
    package: Package,
    versions: Vec<Version>,
    only_version: bool,
    loggedin: bool,
    username: Option<String>,
    is_maintainer: bool,
    api: Option<API>,
    edit: bool,
}

pub fn package_newestver(req: &mut Request) -> IronResult<Response> {
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let id = req.extensions.get::<Router>().unwrap().find("id").unwrap();

    let version = match Package::get(&*connection, id) {
        Ok(pkg) => match pkg.newest_version(&*connection) {
            Ok(pkgver) => pkgver.version(),
            Err(_) => return Ok(Response::with((status::TemporaryRedirect, Redirect({
                let mut base = req.url.clone();
                base.as_mut().path_segments_mut().unwrap().clear();
                base
            })))),
        },
        //most likely the package was not found (TODO match diesel Error as well)
        Err(BackendError::DBError(_)) => return Ok(Response::with((status::TemporaryRedirect, Redirect({
            let mut base = req.url.clone();
            base.as_mut().path_segments_mut().unwrap().clear();
            base
        })))),
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    Ok(Response::with((status::TemporaryRedirect, Redirect({
        let mut base = req.url.clone();
        base.as_mut().path_segments_mut().unwrap().push(&format!("{}", version));
        base
    }))))
}

pub fn package(req: &mut Request) -> IronResult<Response> {
    let ref id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    let ref version = req.extensions.get::<Router>().unwrap().find("version").unwrap();
    let edit = match req.url.path().iter().last() {
        Some(x) if x == &"edit" => true,
        _ => false,
    };

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let pkg = match Package::get(&*connection, id) {
        Ok(pkg) => pkg,
        //most likely the package was not found (TODO match diesel Error as well)
        Err(BackendError::DBError(_)) => return Ok(Response::with((status::TemporaryRedirect, Redirect({
            let mut base = req.url.clone();
            base.as_mut().path_segments_mut().unwrap().clear();
            base
        })))),
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let versions: Vec<Version> = match pkg.versions(&*connection) {
        Ok(versions) => versions.into_iter().sorted_by(|a, b| a.version().cmp(&b.version()).reverse()).map(|x| {
            let mut ver = Version::from(x);
            if ver.version == *version {
                ver.active = true;
            }
            ver
        }).collect(),
        Err(_) => return Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
    };
    debug!("{:?}", versions);

    let only_version = versions.len() == 1;

    let page = match req.extensions.get::<Authenticated>().unwrap() {
        &(true, Some(ref username)) => {
            match User::get(&*connection, username) {
                Ok(user) => {
                    let is_maintainer = match pkg.maintainer(&*connection) {
                        Ok(maintainer) => maintainer == user || user.is_admin(),
                        Err(_) => return Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                    };
                    PackagePage {
                        package: pkg,
                        versions: versions,
                        only_version: only_version,
                        loggedin: true,
                        username: Some(username.clone()),
                        is_maintainer: is_maintainer,
                        edit: edit,
                        api: match user.apikey() {
                            Some(key) => Some(API { key: key }),
                            None => None,
                        },
                    }
                },
                Err(_) => return Ok(Response::with((status::Unauthorized, "User does not exist anymore"))),
            }
        },
        _ => PackagePage {
            package: pkg,
            versions: versions,
            only_version: only_version,
            loggedin: false,
            username: None,
            is_maintainer: false,
            edit: false,
            api: None,
        },
    };

    let mut buffer = Vec::new();
    TEMPLATE.render(&mut buffer, &page).unwrap(); //TODO
    Ok(Response::with((status::Ok, buffer, {
        let mime: Mime = "text/html".parse().unwrap();
        mime
    })))
}
