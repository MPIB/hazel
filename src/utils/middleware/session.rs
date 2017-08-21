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

use iron::{AroundMiddleware, IronResult, Request, Response, Handler, Set};
use iron::headers::{Cookie as CookieHeader, SetCookie};
use iron::modifiers::Header;
use iron::typemap::Key;

use persistent::Write;

use chrono::Duration;
use chrono::prelude::*;

use cookie::{Cookie, CookieJar, Key as CookieKey};

use ::utils::CONFIG;
use ::web::server::SessionStoreKey;

pub struct Authenticated {}
impl Key for Authenticated {
    type Value = (bool, Option<String>);
}

#[derive(Clone)]
pub struct SessionInfo
{
    pub expires: DateTime<Utc>,
    pub session_id: String,
    pub remember: bool,
}

fn handle(req: &mut Request, handler: &Box<Handler>) -> IronResult<Response, > {
    req.extensions.insert::<Authenticated>((false, None));
    if req.url.path().pop().map(|x| x == "login").unwrap_or(false) {
        return handler.handle(req);
    }

    //parse cookies, set auth status
    let cookies = req.headers.get::<CookieHeader>().cloned();
    match cookies {
        Some(cookies) => {
            let mut root_jar = CookieJar::new();
            for cookie in cookies.0 {
                if let Ok(parsed_cookie) = Cookie::parse(cookie) {
                    root_jar.add_original(parsed_cookie);
                }
            }

            {
                let mut jar = root_jar.private(&CookieKey::from_master(&*CONFIG.auth.cookie_key.as_bytes()));

                match jar.get("hazel_username") {
                    Some(mut user_cookie) => {
                        match jar.get("hazel_sessionid") {
                            Some(mut session_cookie) => {
                                let session_store_mutex = req.extensions.get::<Write<SessionStoreKey>>().unwrap().clone();
                                let maybe_session_info = {
                                    let session_store = session_store_mutex.lock().unwrap();
                                    session_store.get(user_cookie.value()).cloned()
                                };
                                match maybe_session_info {
                                    Some(mut session_info) => {
                                        if session_info.session_id == session_cookie.value() {
                                            if session_info.expires >= Utc::now() {
                                                req.extensions.insert::<Authenticated>((true, Some(user_cookie.value().to_string())));
                                                match req.url.path().pop() {
                                                    Some(ref x) if x == &"logout" => {
                                                        jar.remove(session_cookie);
                                                    },
                                                    _ => {
                                                        //renew cookie, if set previously
                                                        match session_info.remember {
                                                            true  => session_info.expires = Utc::now() + Duration::weeks(1),
                                                            false => session_info.expires = Utc::now() + Duration::hours(1),
                                                        };

                                                        let mut session_store = session_store_mutex.lock().unwrap();
                                                        session_store.insert(user_cookie.value().to_string(), session_info.clone());

                                                        session_cookie.set_max_age(session_info.expires.signed_duration_since(Utc::now()));
                                                        user_cookie.set_max_age(session_info.expires.signed_duration_since(Utc::now()));

                                                        session_cookie.set_path(String::from("/"));
                                                        user_cookie.set_path(String::from("/"));

                                                        if let Some(repr) = req.url.as_ref().host_str() {
                                                            session_cookie.set_domain(repr.to_string());
                                                            user_cookie.set_domain(repr.to_string());
                                                        }

                                                        jar.add(session_cookie);
                                                        jar.add(user_cookie);
                                                    }
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
            }

            match handler.handle(req) {
                Ok(resp) => Ok(resp.set(Header(SetCookie(root_jar.delta().map(|cookie| cookie.to_string()).collect())))),
                x => x,
            }
        },
        _ => handler.handle(req),
    }
}

pub struct SessionManager;
impl AroundMiddleware for SessionManager
{
    fn around(self, handler: Box<Handler>) -> Box<Handler>
    {
        Box::new(move |req: &mut Request| {
            handle(req, &handler)
        })
    }
}
