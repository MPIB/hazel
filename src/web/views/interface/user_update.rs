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
use ::utils::middleware::Authenticated;

pub fn update(req: &mut Request) -> IronResult<Response> {

    let params = req.get_ref::<Params>().unwrap().clone();

    let fullname = match params.find(&["fullname"]) {
        Some(&Value::String(ref name)) => name.clone(),
        _ => return Ok(Response::with(Status::BadRequest)),
    };
    let mail = match params.find(&["mail"]) {
        Some(&Value::String(ref mail)) => Some(mail.clone()),
        _ => None,
    };
    let password = match params.find(&["password"]) {
        Some(&Value::String(ref pass)) => if pass.is_empty() { None } else { Some(pass.clone()) },
        _ => None
    };

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match req.extensions.get::<Authenticated>().unwrap()
    {
        &(true, Some(ref username)) =>
            match User::get(&*connection, username) {
                Ok(mut user) => {
                    user.name = fullname;
                    if password.is_some() {
                        if user.update_pass(&*connection, password.unwrap()).is_err() {
                            error!("failed to update password");
                            return Ok(Response::with(Status::InternalServerError));
                        }
                    }
                    if mail.is_some() {
                        if user.set_mail(&*connection, mail.unwrap()).is_err() {
                            error!("failed to update mail");
                            return Ok(Response::with(Status::InternalServerError));
                        }
                    }
                    match user.update(&*connection) {
                        Ok(_) => Ok(Response::with(Status::Ok)),
                        _ => Ok(Response::with(Status::InternalServerError)),
                    }
                },
                _ => Ok(Response::with(Status::Unauthorized)),
            },
        _ => Ok(Response::with(Status::Unauthorized)),
    }
}
