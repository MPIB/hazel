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
use semver::Version;
use regex::Regex;
use treexml::Document;
use ::web::server::ConnectionPoolKey;

use ::web::backend::db::PackageVersion;
use ::web::backend::xml::ToNugetFeedXml;

lazy_static! {
    static ref PKG_DESC: Regex = Regex::new(r#"^Packages\(Id='(?P<id>.*)'\s*,Version='(?P<version>.*)'\)$"#).unwrap();
}

pub fn package(req: &mut Request) -> IronResult<Response> {

    let url = req.url.path().pop().unwrap();
    let parse = match PKG_DESC.captures(&url) {
        Some(matched) => matched,
        None => return Ok(Response::with(status::NotFound)),
    };

    let id = match parse.name("id") {
        Some(matched) => matched,
        None => return Ok(Response::with(status::NotFound)),
    };

    let version = match parse.name("version") {
        Some(matched) => matched,
        None => return Ok(Response::with(status::NotFound)),
    };

    let base_url = {
        let url = &req.url;
        if (&*url.scheme() == "http" && url.port() == 80) || (&*url.scheme() == "https" && url.port() == 443) {
            format!("{}://{}", url.scheme(), url.as_ref().host_str().unwrap())
        } else {
            format!("{}://{}:{}", url.scheme(), url.as_ref().host_str().unwrap(), url.port())
        }
    };
    let connection_pool = req.extensions.get::<Read<ConnectionPoolKey>>().unwrap();

    let connection = match connection_pool.get() {
        Ok(connection) => connection,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let package = match PackageVersion::get(&*connection, id.as_str(), &match Version::parse(version.as_str()) {
        Ok(ver) => ver,
        Err(_) => return Ok(Response::with((status::NotFound, "Version value invalid"))),
    }) {
        Ok(package) => package,
        Err(_) => {
            return Ok(Response::with((status::NotFound, "Package not found")));
        }
    };

    let entry = match package.xml_entry(&*base_url, &*connection) {
        Ok(entry) => entry,
        Err(err) => {
            error!("{:?}", err);
            return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
        }
    };

    let document = Document{
       root: Some(entry),
       .. Document::default()
    };

    Ok(Response::with((status::Ok, format!("{}", document), {
        let mime: Mime = "application/atom+xml".parse().unwrap();
        mime
    })))
}
