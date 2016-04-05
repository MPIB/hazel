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
use plugin::Pluggable;
use params::{Params, Value};

use ::web::server::ConnectionPoolKey;
use ::web::backend::db::{Package, PackageVersion};

use std::str::FromStr;

pub fn complete_ver(req: &mut Request) -> IronResult<Response> {
    let params = req.get_ref::<Params>().unwrap().clone();

    let ref id = req.extensions.get::<Router>().unwrap().find("id").unwrap();

    let include_prerelease = match params.find(&["includePrerelease"]) {
        Some(&Value::Boolean(ref incl)) => *incl,
        Some(&Value::String(ref incl)) => match bool::from_str(incl) {
            Ok(val) => val,
            _ => return Ok(Response::with((status::BadRequest, "includePrerelease is no boolean"))),
        },
        _ => return Ok(Response::with((status::BadRequest, "includePrerelease is no boolean"))),
    };

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();

    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let packages: Vec<PackageVersion> = match Package::get(&*connection, id) {
        Ok(package) => match package.versions(&*connection) {
            Ok(versions) => versions,
            Err(err) => {
                error!("{:?}", err);
                return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
            },
        },
        Err(_) => {
            return Ok(Response::with((status::BadRequest, "Package not found")));
        }
    };

    let mut answer = String::from("[");
    for (i, pkg) in packages.into_iter().filter(|pkgver| if !include_prerelease {
        !pkgver.version().is_prerelease()
    } else { true }).take(30).enumerate() {
        if i != 0 {
            answer.push_str(", ");
        }
        answer.push_str(&format!("{}", pkg.version()));
    }
    answer.push_str("]");

    Ok(Response::with((status::Ok, answer)))
}
