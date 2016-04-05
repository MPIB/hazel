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
use ::web::backend::db::PackageVersion;

pub fn download(req: &mut Request) -> IronResult<Response> {
    let storage = req.extensions.get::<Read<StorageKey>>().unwrap();
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();

    let ref id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    let ref version = req.extensions.get::<Router>().unwrap().find("version").unwrap();

    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

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
        }
    };

    match pkgver.download(storage) {
        Ok(file) => Ok(Response::with((status::Ok, file))),
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Filesystem Error, please try again later")));
        }
    }
}
