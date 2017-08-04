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
use iron::mime::Mime;
use persistent::Read;
use mustache::{Template, compile_path};

use std::path::PathBuf;

use ::web::server::ConnectionPoolKey;
use ::utils::CONFIG;
use ::utils::middleware::Authenticated;
use ::web::backend::db::User;

lazy_static! {
    static ref TEMPLATE: Template = compile_path(PathBuf::from(CONFIG.web.resources.clone()).join("user.html")).unwrap();
}

#[derive(Serialize)]
struct UserPage
{
    user: User,
    mail: bool,
    edit: bool,
    plainauth: bool,
}

pub fn user(req: &mut Request) -> IronResult<Response> {
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let user = match req.extensions.get::<Authenticated>().unwrap() {
        &(true, Some(ref username)) => {
            match User::get(&*connection, username) {
                Ok(user) => user,
                Err(_) => return Ok(Response::with((status::Unauthorized, "User does not exist anymore"))),
            }
        },
        _ => return Ok(Response::with(status::Unauthorized)),
    };

    let plain = user.is_plainauth();
    let mail = user.mail().is_some();

    let rendering = UserPage {
        user: user,
        mail: mail,
        edit: match req.url.path().iter().last() {
            Some(x) if x == &"edit" => true,
            _ => false,
        },
        plainauth: plain,
    };

    let mut buffer = Vec::new();
    TEMPLATE.render(&mut buffer, &rendering).unwrap();
    Ok(Response::with((status::Ok, buffer, {
        let mime: Mime = "text/html".parse().unwrap();
        mime
    })))
}
