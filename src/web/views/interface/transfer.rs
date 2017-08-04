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
use ::utils::error::BackendError;

use ::web::server::ConnectionPoolKey;
use ::utils::middleware::Authenticated;
use ::web::backend::db::{User, Package};

pub fn transfer(req: &mut Request) -> IronResult<Response> {
    let ref id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    let ref new_owner = req.extensions.get::<Router>().unwrap().find("new_maintainer").unwrap();

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let mut pkg = match Package::get(&*connection, id) {
        Ok(pkg) => pkg,
        //most likely the package was not found (TODO match diesel Error as well)
        Err(BackendError::DBError(_)) => return Ok(Response::with((status::NotFound, "Package not found"))),
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match req.extensions.get::<Authenticated>().unwrap() {
        &(true, Some(ref username)) => {
            match User::get(&*connection, username) {
                Ok(user) => {
                    let is_maintainer = match pkg.maintainer(&*connection) {
                        Ok(maintainer) => maintainer == user || user.is_admin(),
                        Err(_) => return Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                    };

                    if is_maintainer {

                        match User::get(&*connection, &String::from(*new_owner)) {
                            Ok(new_maintainer) =>
                                if new_maintainer.confirmed() {
                                    match pkg.update_maintainer(&*connection, &new_maintainer) {
                                        Ok(_) => Ok(Response::with(status::Ok)),
                                        Err(err) => {
                                            error!("{:?}", err);
                                            Ok(Response::with((status::InternalServerError, "Transfer failed")))
                                        },
                                    }
                                } else {
                                    Ok(Response::with((status::BadRequest, "The new maintainer is not confirmed yet. Not transferring")))
                                },
                            Err(_) => Ok(Response::with((status::BadRequest, "The new maintainer does not exist. Not transferring"))),
                        }

                    } else {
                        Ok(Response::with((status::Forbidden, "You are not the maintainer of the requested package.")))
                    }
                },
                Err(_) => Ok(Response::with((status::Unauthorized, "User does not exist anymore"))),
            }
        },
        _ => Ok(Response::with(status::Unauthorized)),
    }
}
