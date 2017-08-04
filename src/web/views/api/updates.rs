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
use chrono::prelude::*;
use semver::Version;
use plugin::Pluggable;
use params::{Params, Value};
use treexml::{Document, Element};
use ::web::server::ConnectionPoolKey;
use ::web::backend::db::{Package, PackageVersion};
use ::web::backend::xml::ToNugetFeedXml;
use std::str::FromStr;

pub fn updates(req: &mut Request) -> IronResult<Response> {

    let params = req.get_ref::<Params>().unwrap().clone();

    let ids: Vec<(&str, &str)> = match params.find(&["packageids"]) {
        Some(&Value::String(ref id)) => id.split('|'),
        _ => return Ok(Response::with((status::BadRequest, "packageids is no String"))),
    }.zip(match params.find(&["versions"]) {
        Some(&Value::String(ref id)) => id.split('|'),
        _ => return Ok(Response::with((status::BadRequest, "versions is no String"))),
    }).collect();

    let include_all_versions = match params.find(&["includeAllVersions"]) {
        Some(&Value::Boolean(ref incl)) => *incl,
        Some(&Value::String(ref incl)) => match bool::from_str(incl) {
            Ok(val) => val,
            _ => return Ok(Response::with((status::BadRequest, "includeAllVersions is no boolean"))),
        },
        _ => return Ok(Response::with((status::BadRequest, "includeAllVersions is no boolean"))),
    };

    let include_prerelease = match params.find(&["includePrerelease"]) {
        Some(&Value::Boolean(ref incl)) => *incl,
        Some(&Value::String(ref incl)) => match bool::from_str(incl) {
            Ok(val) => val,
            _ => return Ok(Response::with((status::BadRequest, "includePrerelease is no boolean"))),
        },
        _ => return Ok(Response::with((status::BadRequest, "includePrerelease is no boolean"))),
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

    let packages: Vec<PackageVersion> = {
        let mut packages = Vec::new();
        for (pkg_id, cur_version) in ids {
            let cur_version = match Version::parse(cur_version) {
                Ok(ver) => ver,
                Err(_) => return Ok(Response::with((status::BadRequest, "Version value invalid"))),
            };
            let pkg = match Package::get(&*connection, pkg_id) {
                Ok(pkg) => pkg,
                Err(err) => {
                    error!("{:?}", err);
                    return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
                }
            };
            let versions = match pkg.versions(&*connection) {
                Ok(versions) => {
                    let cur_version = cur_version.clone();
                    if !include_prerelease {
                        Box::new(versions.into_iter().filter(|pkgver| pkgver.version().is_prerelease())) as  Box<Iterator<Item=PackageVersion>>
                    } else {
                        Box::new(versions.into_iter()) as  Box<Iterator<Item=PackageVersion>>
                    }.filter(move |pkgver| pkgver.version() > cur_version)
                },
                Err(err) => {
                    error!("{}", err);
                    return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
                }
            };
            packages.push(
                if !include_all_versions {
                    Box::new(versions.max_by_key(|pkgver| pkgver.version()).into_iter()) as  Box<Iterator<Item=PackageVersion>>
                } else {
                    Box::new(versions) as  Box<Iterator<Item=PackageVersion>>
                }
            );
        }
        packages.into_iter().flat_map(|x| x).collect()
    };

    let mut feed = Element::new("feed");
    feed.attributes.insert(String::from("xml:base"), format!("{}/api/v2/", &*base_url));
    feed.attributes.insert(String::from("xmlns:d"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices"));
    feed.attributes.insert(String::from("xmlns:m"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices/metadata"));
    feed.attributes.insert(String::from("xmlns"), String::from("http://www.w3.org/2005/Atom"));

    let mut title = Element::new("title");
    title.text = Some(String::from("FindPackagesById"));
    feed.children.push(title);

    let mut id = Element::new("id");
    id.text = Some(format!("{}/api/v2/FindPackagesById", &*base_url));
    feed.children.push(id);

    let mut last_updated = Utc.ymd(1900, 1, 1).and_hms(0, 0, 0).naive_utc();
    for pkg in packages.iter()
    {
        if pkg.last_updated() > &last_updated {
            last_updated = pkg.last_updated().clone();
        }
    }
    let mut updated = Element::new("updated");
    updated.text = Some(format!("{:?}Z", last_updated));
    feed.children.push(updated);

    let mut link = Element::new("link");
    link.attributes.insert(String::from("title"), String::from("FindPackagesById"));
    link.attributes.insert(String::from("href"), String::from("FindPackagesById"));
    feed.children.push(link);

    for pkg in packages.into_iter()
    {
        feed.children.push(match pkg.xml_entry(&*base_url, &*connection) {
            Ok(entry) => entry,
            Err(err) => {
                error!("{:?}", err);
                return Ok(Response::with((status::InternalServerError, "Database Error, please try again later")));
            }
        });
    }

    //TODO when we limit we need to generate a continuation link, also we need to map this in the server
    //e.g. <link rel="next" href="http://chocolatey.org/api/v2/Packages?$skiptoken='1password','1.0.9.332'" />

    let document = Document{
       root: Some(feed),
       .. Document::default()
    };

    Ok(Response::with((status::Ok, format!("{}", document), {
        let mime: Mime = "application/atom+xml".parse().unwrap();
        mime
    })))
}
