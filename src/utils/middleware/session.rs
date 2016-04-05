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

use iron::{AroundMiddleware, Request, Handler, Set};
use iron::headers::{Cookie, SetCookie};
use iron::modifiers::Header;
use iron::typemap::Key;

use persistent::Write;

use chrono::*;

use ::utils::CONFIG;
use ::web::server::SessionStoreKey;

pub struct Authenticated {}
impl Key for Authenticated {
    type Value = (bool, Option<String>);
}

#[derive(Clone)]
pub struct SessionInfo
{
    pub expires: DateTime<UTC>,
    pub session_id: String,
    pub remember: bool,
}

pub struct SessionManager;
impl AroundMiddleware for SessionManager
{
    fn around(self, handler: Box<Handler>) -> Box<Handler>
    {
        Box::new(move |req: &mut Request| {
            req.extensions.insert::<Authenticated>((false, None));

            //parse cookies, set auth status
            let cookies = req.headers.get::<Cookie>().cloned();
            match cookies {
                Some(header) => {
                    let root_jar = header.to_cookie_jar(&*CONFIG.auth.cookie_key.as_bytes());
                    let jar = root_jar.encrypted();
                    match jar.find("hazel_username") {
                        Some(mut user_cookie) => {
                            match jar.find("hazel_sessionid") {
                                Some(mut session_cookie) => {
                                    let session_store_mutex = req.extensions.get::<Write<SessionStoreKey>>().unwrap().clone();
                                    let maybe_session_info = {
                                        let session_store = session_store_mutex.lock().unwrap();
                                        session_store.get(&user_cookie.value).cloned()
                                    };
                                    match maybe_session_info {
                                        Some(mut session_info) => {
                                            if session_info.session_id == session_cookie.value {
                                                if session_info.expires >= UTC::now() {
                                                    req.extensions.insert::<Authenticated>((true, Some(user_cookie.value.clone())));
                                                    match handler.handle(req) {
                                                        Ok(resp) => {
                                                            match req.url.path.pop() {
                                                                Some(ref x) if x == "logout" => {},
                                                                _ => {
                                                                    //renew cookie, if set previously
                                                                    match session_info.remember {
                                                                        true  => session_info.expires = UTC::now() + Duration::weeks(1),
                                                                        false => session_info.expires = UTC::now() + Duration::hours(1),
                                                                    };

                                                                    let mut session_store = session_store_mutex.lock().unwrap();
                                                                    session_store.insert(user_cookie.value.clone(), session_info.clone());

                                                                    session_cookie.max_age = Some((session_info.expires - UTC::now()).num_seconds() as u64);
                                                                    user_cookie.max_age = Some((session_info.expires - UTC::now()).num_seconds() as u64);

                                                                    session_cookie.path = Some(String::from("/"));
                                                                    user_cookie.path = Some(String::from("/"));

                                                                    session_cookie.domain = Some(req.url.host.to_string());
                                                                    user_cookie.domain = Some(req.url.host.to_string());

                                                                    jar.add(session_cookie);
                                                                    jar.add(user_cookie);
                                                                    return Ok(resp.set(Header(SetCookie::from_cookie_jar(&root_jar))));
                                                                }
                                                            }
                                                        },
                                                        Err(x) => return Err(x),
                                                    }
                                                }
                                            }
                                        },
                                        None => {},
                                    }
                                },
                                None => {},
                            }
                        },
                        None => {},
                    }
                },
                None => {},
            };

            handler.handle(req)
        })
    }
}
