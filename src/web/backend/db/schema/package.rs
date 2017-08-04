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

use diesel::prelude::*;
use diesel::pg::Pg;
use diesel::{update, delete};
use lazysort::SortedBy;

use ::utils::error::{BackendError, BackendResult};
use ::web::backend::db::{package, package_has_tag, tag, packageversion};
use ::web::backend::db::schema::{User, PackageHasTag, Tag, PackageVersion};
use ::web::backend::Storage;

#[derive(Queryable, Debug, Insertable, Identifiable, AsChangeset, Serialize)]
#[table_name="package"]
pub struct Package
{
    pub id: String,
    pub project_url: Option<String>,
    pub license_url: Option<String>,
    pub license_acceptance: bool,
    pub project_source_url: Option<String>,
    pub package_source_url: Option<String>,
    pub docs_url: Option<String>,
    pub mailing_list_url: Option<String>,
    pub bug_tracker_url: Option<String>,
    pub report_abuse_url: Option<String>,
    pub maintainer: String,
}

impl Package
{
    pub fn new(id: String, maintainer: &User) -> Self
    {
        Package {
            id: id,
            project_url: None,
            license_url: None,
            license_acceptance: false,
            project_source_url: None,
            package_source_url: None,
            docs_url: None,
            mailing_list_url: None,
            bug_tracker_url: None,
            report_abuse_url: None,
            maintainer: maintainer.id.clone(),
        }
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, id: &str) -> BackendResult<Self>
    {
        err!(package::table.filter(package::id.eq(id)).first(connection))
    }

    pub fn all<C: Connection<Backend=Pg>>(connection: &C) -> BackendResult<Vec<Self>>
    {
        err!(package::table.load(connection))
    }

    pub fn update<C: Connection<Backend=Pg>>(&self, connection: &C, storage: &Storage) -> BackendResult<Self>
    {
        let result = try!(update(package::table.filter(package::id.eq(&self.id))).set(self).get_result(connection));
        for ver in try!(self.versions(connection)) {
            try!(ver.update(connection, storage));
        }
        Ok(result)
    }

    pub fn maintainer<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<User>
    {
        User::get(connection, &self.maintainer)
    }

    pub fn update_maintainer<C: Connection<Backend=Pg>>(&mut self, connection: &C, maintainer: &User) -> BackendResult<Self>
    {
        self.maintainer = maintainer.id.clone();
        err!(update(package::table.filter(package::id.eq(&self.id))).set(self as &Package).get_result(connection))
    }

    pub fn delete<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<()>
    {
        connection.transaction(|| {
            for tag in try!(self.tags(connection)) {
                try!(tag.disconnect(connection, &self));
            }
            err_discard!(delete(package::table.filter(package::id.eq(&self.id))).execute(connection))
        })
    }

    pub fn tags<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Tag>>
    {
        let package_has_tags: Vec<PackageHasTag> = try!(package_has_tag::table.filter({
            package_has_tag::package_id.eq(&self.id)
        }).load(connection));

        let mut iterators = Vec::new();

        for package_has_tag_entry in package_has_tags.into_iter()
        {
            iterators.push(try!(tag::table.filter(tag::id.eq(package_has_tag_entry.id)).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    pub fn versions<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<PackageVersion>>
    {
        err!(packageversion::table.filter(packageversion::id.eq(&self.id)).load(connection))
    }

    pub fn newest_version<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<PackageVersion>
    {
        match try!(self.versions(&*connection)).into_iter().sorted_by(|a, b| a.version().cmp(&b.version()).reverse()).next() {
            Some(ver) => Ok(ver),
            None => Err(BackendError::NotFound),
        }
    }

    pub fn id(&self) -> &str
    {
        &self.id
    }
}

impl PartialEq for Package
{
    fn eq(&self, other: &Self) -> bool
    {
        self.id == other.id
    }
}
impl Eq for Package {}
