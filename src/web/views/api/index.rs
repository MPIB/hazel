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

pub fn index(req: &mut Request) -> IronResult<Response> {
    let base_url = {
        let url = &req.url;
        if (&*url.scheme() == "http" && url.port() == 80) || (&*url.scheme() == "https" && url.port() == 443) {
            format!("{}://{}", url.scheme(), url.as_ref().host_str().unwrap())
        } else {
            format!("{}://{}:{}", url.scheme(), url.as_ref().host_str().unwrap(), url.port())
        }
    };
    Ok(Response::with((status::Ok, format!(
"<service xmlns:atom=\"http://www.w3.org/2005/Atom\" xmlns:app=\"http://www.w3.org/2007/app\" xmlns=\"http://www.w3.org/2007/app\" xml:base=\"{}/api/v2/\">
<workspace>
    <atom:title>Default</atom:title>
    <collection href=\"Packages\">
        <atom:title>Packages</atom:title>
    </collection>
</workspace>
</service>"
    , &*base_url), {
        let mime: Mime = "application/atom+xml".parse().unwrap();
        mime
    })))
}
