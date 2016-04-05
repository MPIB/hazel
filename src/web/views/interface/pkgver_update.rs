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

use iron::{Request, Response, IronResult, Plugin};
use iron::status::{self, Status};
use router::Router;
use semver::Version;
use params::{Params, Value};
use persistent::Read;

use ::web::backend::db::{User, PackageVersion};
use ::web::server::{ConnectionPoolKey, StorageKey};
use ::utils::error::BackendError;
use ::utils::middleware::Authenticated;

pub fn pkgver_update(req: &mut Request) -> IronResult<Response> {

    let id = match req.extensions.get::<Router>().unwrap().find("id").map(|x| String::from(x)) {
        Some(id) => id,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let version = match req.extensions.get::<Router>().unwrap().find("version").map(|x| String::from(x)) {
        Some(version) => version,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let params = req.get_ref::<Params>().unwrap().clone();

    let summary = match params.find(&["summary"]) {
        Some(&Value::String(ref summary)) => Some(summary.clone()),
        _ => None,
    };
    let description = match params.find(&["description"]) {
        Some(&Value::String(ref description)) => Some(description.clone()),
        _ => None,
    };
    let release_notes = match params.find(&["release_notes"]) {
        Some(&Value::String(ref release_notes)) => Some(release_notes.clone()),
        _ => None,
    };

    let storage = req.extensions.get::<Read<StorageKey>>().unwrap();
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let mut pkgver = match PackageVersion::get(&*connection, &*id, &match Version::parse(&*version) {
        Ok(ver) => ver,
        Err(_) => return Ok(Response::with((status::UnprocessableEntity, "Version value invalid"))),
    }) {
        Ok(pkgver) => pkgver,
        //most likely the package was not found (TODO match diesel Error as well)
        Err(BackendError::DBError(_)) => return Ok(Response::with((status::NotFound, "Package/Version not found"))),
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let pkg = match pkgver.package(&*connection) {
        Ok(pkg) => pkg,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match req.extensions.get::<Authenticated>().unwrap()
    {
        &(true, Some(ref username)) =>
            match User::get(&*connection, username) {
                Ok(user) => {
                    let is_maintainer = match pkg.maintainer(&*connection) {
                        Ok(maintainer) => maintainer == user || user.is_admin(),
                        Err(_) => return Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                    };

                    if is_maintainer {

                        //not really the best rust syntax
                        //TODO style
                        let mut flag = false;
                        if summary.is_some() { pkgver.summary = summary; flag = true; }
                        if description.is_some() { pkgver.description = description; flag = true; };
                        if release_notes.is_some() { pkgver.release_notes = release_notes; flag = true; };

                        if flag { match pkgver.update(&*connection, storage) {
                            Ok(_) => Ok(Response::with(status::Ok)),
                            //TODO match critical storage error (deletion)
                            Err(_) => Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                        } } else {
                            Ok(Response::with(status::Ok))
                        }
                    } else {
                        Ok(Response::with((status::Forbidden, "You are not the maintainer of the requested package.")))
                    }
                },
                _ => Ok(Response::with(Status::Unauthorized)),
            },
        _ => Ok(Response::with(Status::Unauthorized)),
    }
}
