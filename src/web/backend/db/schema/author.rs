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
use diesel::{insert, delete};

use ::utils::error::BackendResult;
use ::web::backend::db::{author, packageversion, packageversion_has_author};
use ::web::backend::db::schema::{PackageVersion, PackageVersionHasAuthor};

#[derive(Queryable, Debug, PartialEq, Eq, Insertable)]
#[table_name="author"]
pub struct Author
{
    pub id: String
}

impl Author
{
    pub fn new<C: Connection<Backend=Pg>>(connection: &C,
               package_version: &PackageVersion,
               name: String,
              ) -> BackendResult<Self>
    {
        let this = Author {
            id: name
        };

        let result = try!(insert(&this).into(author::table).get_result(connection));
        try!(PackageVersionHasAuthor::new(connection, package_version, &this));
        Ok(result)
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, id: &str) -> BackendResult<Self>
    {
        err!(author::table.filter(
                author::id.eq(id)
            ).first(connection))
    }

    pub fn belongs<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<PackageVersion>>
    {
        let packageversion_has_authors: Vec<PackageVersionHasAuthor> = try!(packageversion_has_author::table.filter({
            packageversion_has_author::author_id.eq(&self.id)
        }).load(connection));

        let mut iterators = Vec::new();

        for packageversion_has_author_entry in packageversion_has_authors.into_iter()
        {
            iterators.push(try!(packageversion::table.filter(
                packageversion::id.eq(packageversion_has_author_entry.id)
                .and(packageversion::version.eq(packageversion_has_author_entry.version))
            ).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    pub fn connect<C: Connection<Backend=Pg>>(&self, connection: &C, version: &PackageVersion) -> BackendResult<()>
    {
        match PackageVersionHasAuthor::get(connection, version, self) {
            Ok(_) => Ok(()),
            Err(_) => match PackageVersionHasAuthor::new(connection, version, self) {
                Ok(_) => Ok(()),
                Err(x) => Err(x),
            },
        }
    }

    pub fn disconnect<C: Connection<Backend=Pg>>(self, connection: &C, version: &PackageVersion) -> BackendResult<()>
    {
        match PackageVersionHasAuthor::get(connection, version, &self) {
            Ok(has) => {
                connection.transaction(move || {
                    try!(has.delete(connection));
                    if try!(self.belongs(connection)).len() == 0 { try!(self.delete(connection)) };
                    Ok(())
                })
            },
            Err(x) => Err(x),
        }
    }

    pub fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(author::table.filter(author::id.eq(self.id))).execute(connection))
    }

    pub fn name(&self) -> &str
    {
        &self.id
    }
}
