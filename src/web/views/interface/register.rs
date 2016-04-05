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
use params::{Params, Value};
use persistent::Read;

use ::web::backend::db::User;
use ::web::server::ConnectionPoolKey;
use ::utils::error::BackendError;
use ::utils::CONFIG;

pub fn register(req: &mut Request) -> IronResult<Response> {

    if !CONFIG.auth.open_for_registration {
        return Ok(Response::with(Status::BadRequest));
    }

    let params = req.get_ref::<Params>().unwrap().clone();

    let username = match params.find(&["username"]) {
        Some(&Value::String(ref name)) => name.clone(),
        _ => return Ok(Response::with(Status::BadRequest)),
    };
    let fullname = match params.find(&["fullname"]) {
        Some(&Value::String(ref name)) => name.clone(),
        _ => return Ok(Response::with(Status::BadRequest)),
    };
    let mail = match params.find(&["mail"]) {
        Some(&Value::String(ref mail)) => mail.clone(),
        _ => return Ok(Response::with(Status::BadRequest)),
    };
    let password = match params.find(&["password"]) {
        Some(&Value::String(ref pass)) => pass.clone(),
        _ => return Ok(Response::with(Status::BadRequest)),
    };

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match User::register(&*connection, username, fullname, mail, password) {
        Ok(_) => Ok(Response::with(Status::Ok)),
        Err(BackendError::UserAlreadyExists) | Err(BackendError::DBError(_)) => Ok(Response::with((Status::BadRequest, "User already exists"))),
        Err(x) => { error!("{:?}", x); Ok(Response::with((Status::InternalServerError, "Please try again later"))) },
    }
}
