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
use iron::modifiers::Redirect;
use persistent::Read;
use router::Router;

use ::web::server::ConnectionPoolKey;
use ::web::backend::db::User;
use ::utils::error::BackendError;
use diesel::result::Error as DBError;

pub fn mail_confirmation(req: &mut Request) -> IronResult<Response>
{
    let ref key = req.extensions.get::<Router>().unwrap().find("key").unwrap();

    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match User::confirm_mail(&*connection, String::from(*key)) {
        Ok(_) => Ok(Response::with((status::TemporaryRedirect, Redirect({
            let mut base = req.url.clone();
            base.as_mut().path_segments_mut().unwrap().clear();
            base
        })))),
        Err(BackendError::DBError(DBError::NotFound)) => Ok(Response::with(status::NotFound)),
        Err(_) => Ok(Response::with(status::InternalServerError)),
    }
}
