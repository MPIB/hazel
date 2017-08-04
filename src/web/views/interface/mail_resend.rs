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
use iron::status::Status;
use persistent::Read;

use ::web::backend::db::User;
use ::web::server::ConnectionPoolKey;
use ::utils::middleware::Authenticated;

pub fn mail_resend(req: &mut Request) -> IronResult<Response>
{
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((Status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match req.extensions.get::<Authenticated>().unwrap()
    {
        &(true, Some(ref username)) =>
            match User::get(&*connection, username) {
                Ok(user) => {
                    match user.send_mail() {
                        Ok(_) => Ok(Response::with(Status::Ok)),
                        Err(_) => Ok(Response::with(Status::InternalServerError)),
                    }
                },
                _ => Ok(Response::with(Status::Unauthorized)),
            },
        _ => Ok(Response::with(Status::Unauthorized)),
    }
}
