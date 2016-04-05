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
use persistent::Read;
use router::Router;
use semver::Version;
use ::utils::error::BackendError;
use ::web::server::{ConnectionPoolKey, StorageKey};
use ::web::backend::db::{Package, PackageVersion, User};

header! { (XNugetApiKey, "X-NuGet-ApiKey") => [String] }

pub fn delete(req: &mut Request) -> IronResult<Response> {
    let storage = req.extensions.get::<Read<StorageKey>>().unwrap();
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();

    let apikey = match req.headers.get::<XNugetApiKey>().cloned() {
        Some(key) => key.0,
        None => return Ok(Response::with((status::Unauthorized))),
    };

    let ref id = req.extensions.get::<Router>().unwrap().find("id").unwrap();

    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match User::get_by_apikey(&*connection, &apikey) {
        Ok(user) => {
            match req.extensions.get::<Router>().unwrap().find("version") {
                Some(version) => {
                    let pkgver = match PackageVersion::get(&*connection, id, &match Version::parse(version) {
                        Ok(ver) => ver,
                        Err(_) => return Ok(Response::with((status::UnprocessableEntity, "Version value invalid"))),
                    }) {
                        Ok(pkgver) => pkgver,
                        //mostlikely the package was not found (TODO match diesel Error as well)
                        Err(BackendError::DBError(_)) => return Ok(Response::with((status::NotFound, "Package not found"))),
                        Err(err) => {
                            error!("{:?}", err);
                            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
                        },
                    };

                    let maintainer = match pkgver.package(&*connection) {
                        Ok(pkg) => match pkg.maintainer(&*connection) {
                            Ok(maintainer) => maintainer,
                            Err(err) => {
                                error!("{:?}", err);
                                return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")))
                            },
                        },
                        Err(err) => {
                            error!("{:?}", err);
                            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")))
                        },
                    };

                    if user.is_admin() || maintainer == user {
                        match pkgver.delete(&*connection, storage) {
                            Ok(()) => Ok(Response::with(status::Ok)),
                            Err(err) => {
                                error!("{:?}", err);
                                Ok(Response::with((status::InternalServerError, "Database Error, please try again later")))
                            },
                        }
                    } else {
                        Ok(Response::with((status::Forbidden, "Only the maintainer or admin is allowed to delete a package")))
                    }
                },
                None => {
                    let pkg = match Package::get(&*connection, id) {
                        Ok(pkg) => pkg,
                        //mostlikely the package was not found (TODO match diesel Error as well)
                        Err(BackendError::DBError(_)) => return Ok(Response::with((status::NotFound, "Package not found"))),
                        Err(err) => {
                            error!("{:?}", err);
                            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
                        },
                    };

                    let maintainer = match pkg.maintainer(&*connection) {
                        Ok(maintainer) => maintainer,
                        Err(err) => {
                            error!("{:?}", err);
                            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")))
                        },
                    };

                    if user.is_admin() || maintainer == user {
                        match pkg.versions(&*connection) {
                            Ok(versions) => {
                                for version in versions {
                                    match version.delete(&*connection, storage) {
                                        Ok(()) => {},
                                        Err(err) => {
                                            error!("{:?}", err);
                                            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
                                        }
                                    }
                                }
                                Ok(Response::with(status::Ok))
                            },
                            Err(err) => {
                                error!("{:?}", err);
                                Ok(Response::with((status::InternalServerError, "Database Error, please try again later")))
                            }
                        }
                    } else {
                        Ok(Response::with((status::Forbidden, "Only the maintainer or admin is allowed to delete a package")))
                    }
                }
            }
        }
        //TODO better match
        Err(_) => Ok(Response::with((status::InternalServerError, "No User with matching API-Key found"))),
    }
}
