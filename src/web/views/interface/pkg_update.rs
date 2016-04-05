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
use router::Router;
use params::{Params, Value};
use persistent::Read;

use ::web::backend::db::{User, Package};
use ::web::server::{ConnectionPoolKey, StorageKey};
use ::utils::middleware::Authenticated;

pub fn pkg_update(req: &mut Request) -> IronResult<Response> {

    let id = match req.extensions.get::<Router>().unwrap().find("id").map(|x| String::from(x)) {
        Some(id) => id,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let params = req.get_ref::<Params>().unwrap().clone();

    let project_url = match params.find(&["project_url"]) {
        Some(&Value::String(ref project_url)) => Some(project_url.clone()),
        _ => None,
    };
    let license_url = match params.find(&["license_url"]) {
        Some(&Value::String(ref license_url)) => Some(license_url.clone()),
        _ => None,
    };
    let project_source_url = match params.find(&["project_source_url"]) {
        Some(&Value::String(ref project_source_url)) => Some(project_source_url.clone()),
        _ => None,
    };
    let package_source_url = match params.find(&["package_source_url"]) {
        Some(&Value::String(ref package_source_url)) => Some(package_source_url.clone()),
        _ => None,
    };
    let docs_url = match params.find(&["docs_url"]) {
        Some(&Value::String(ref docs_url)) => Some(docs_url.clone()),
        _ => None,
    };
    let mailing_list_url = match params.find(&["mailing_list_url"]) {
        Some(&Value::String(ref mailing_list_url)) => Some(mailing_list_url.clone()),
        _ => None,
    };
    let bug_tracker_url = match params.find(&["bug_tracker_url"]) {
        Some(&Value::String(ref bug_tracker_url)) => Some(bug_tracker_url.clone()),
        _ => None,
    };
    let report_abuse_url = match params.find(&["report_abuse_url"]) {
        Some(&Value::String(ref report_abuse_url)) => Some(report_abuse_url.clone()),
        _ => None,
    };

    let storage = req.extensions.get::<Read<StorageKey>>().unwrap();
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();
    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let mut pkg = match Package::get(&*connection, &*id) {
        Ok(pkg) => pkg,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    match req.extensions.get::<Authenticated>().unwrap()
    {
        &(true, Some(ref username)) =>
            match User::get(&*connection, username) {
                Ok(user) => {
                    let is_maintainer = match pkg.maintainer(&*connection) {
                        Ok(maintainer) => maintainer == user || user.is_admin(),
                        Err(_) => return Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                    };

                    if is_maintainer {

                        //not really the best rust syntax
                        //TODO style
                        let mut flag = false;
                        if project_url.is_some() { pkg.project_url = project_url; flag = true; }
                        if license_url.is_some() { pkg.license_url = license_url; flag = true; }
                        if project_source_url.is_some() { pkg.project_source_url = project_source_url; flag = true; }
                        if package_source_url.is_some() { pkg.package_source_url = package_source_url; flag = true; }
                        if docs_url.is_some() { pkg.docs_url = docs_url; flag = true; }
                        if mailing_list_url.is_some() { pkg.mailing_list_url = mailing_list_url; flag = true; }
                        if bug_tracker_url.is_some() { pkg.bug_tracker_url = bug_tracker_url; flag = true; }
                        if report_abuse_url.is_some() { pkg.report_abuse_url = report_abuse_url; flag = true; }

                        if flag { match pkg.update(&*connection, storage) {
                            Ok(_) => Ok(Response::with(status::Ok)),
                            //TODO match critical storage error (deletion)
                            Err(_) => Ok(Response::with((status::InternalServerError, "Database Error, please try again later"))),
                        } } else {
                            Ok(Response::with(status::Ok))
                        }
                    } else {
                        Ok(Response::with((status::Forbidden, "You are not the maintainer of the requested package.")))
                    }
                },
                _ => Ok(Response::with(Status::Unauthorized)),
            },
        _ => Ok(Response::with(Status::Unauthorized)),
    }
}
