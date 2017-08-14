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
use multipart::server::Entries;

use std::fs::File;

use ::web::server::{ConnectionPoolKey, StorageKey};
use ::web::backend::db::{PackageVersion, User};
use ::utils::error::BackendError;

header! { (XNugetApiKey, "X-NuGet-ApiKey") => [String] }

pub fn upload(req: &mut Request) -> IronResult<Response> {
    let params = req.extensions.get::<Entries>().unwrap().clone();

    let apikey_raw = req.headers.get_raw("X-NuGet-ApiKey").expect("No Raw API Key");
    info!("Api Key: {:?}", apikey_raw);
    let apikey = req.headers.get::<XNugetApiKey>().cloned().expect("No API Key").0;

    let storage = req.extensions.get::<Read<StorageKey>>().unwrap();
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(x) => {
            error!("Database Error: {:?}", x);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match User::get_by_apikey(&*connection, &apikey) {
        Ok(user) => {
            match params.files.get("package").and_then(|x| x.get(0)) {
                Some(file) => {
                    match PackageVersion::new(&*connection, &user, storage, File::open(&file.path).unwrap()) {
                        Ok(_) => Ok(Response::with(status::Ok)),
                        Err(BackendError::PermissionDenied) => Ok(Response::with((status::Forbidden, "Only the maintainer or admin is allowed to update a package"))),
                        Err(err) => {
                            error!("Permission Error: {}", err);
                            Ok(Response::with((status::BadRequest, format!("{}", err))))
                        },
                    }
                },
               _ => {
                   error!("Package is no file");
                   Ok(Response::with((status::BadRequest, "package is no File")))
                },
            }
        },
        //TODO better match
        Err(x) => {
            error!("No user: {}", x);
            Ok(Response::with((status::InternalServerError, "No User with matching API-Key found")))
        },
    }
}
