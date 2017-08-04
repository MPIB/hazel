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
use plugin::Pluggable;
use params::{Params, Value};
use treexml::{Document, Element};
use ::web::server::ConnectionPoolKey;
use ::web::backend::db::Package;
use ::web::backend::xml::ToNugetFeedXml;

pub fn packagesbyid(req: &mut Request) -> IronResult<Response> {

    let params = req.get_ref::<Params>().unwrap().clone();

    let trimmer: &[_] = &['\\', '"', '\''];
    let ref id = match params.find(&["id"]) {
        Some(&Value::String(ref term)) => term.trim_matches(trimmer),
        _ => return Ok(Response::with((status::BadRequest, "id is no String"))),
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

    let package = match Package::get(&*connection, id) {
        Ok(package) => package,
        Err(_) => {
            return Ok(Response::with((status::NotFound, "Package not found")));
        }
    };

    let packages = match package.versions(&*connection) {
        Ok(pkgs) => pkgs,
        Err(_) => {
            return Ok(Response::with((status::NotFound, "Package not found")));
        }
    };

    let mut feed = Element::new("feed");
    feed.attributes.insert(String::from("xml:base"), format!("{}/api/v2/", base_url));
    feed.attributes.insert(String::from("xmlns:d"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices"));
    feed.attributes.insert(String::from("xmlns:m"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices/metadata"));
    feed.attributes.insert(String::from("xmlns"), String::from("http://www.w3.org/2005/Atom"));

    let mut title = Element::new("title");
    title.text = Some(String::from("FindPackagesById"));
    feed.children.push(title);

    let mut id = Element::new("id");
    id.text = Some(format!("{}/api/v2/FindPackagesById", base_url));
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
