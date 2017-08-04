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
use ::web::backend::db::packageversion_has_author;
use ::web::backend::db::schema::{Author, PackageVersion};

#[derive(Queryable, Debug, Insertable, Identifiable, AsChangeset)]
#[table_name = "packageversion_has_author"]
pub struct PackageVersionHasAuthor
{
    pub id: String,
    pub version: String,
    pub author_id: String,
}

impl PackageVersionHasAuthor
{
    pub fn new<C: Connection<Backend=Pg>>(connection: &C, package_version: &PackageVersion, author: &Author) -> BackendResult<Self>
    {
        let this = PackageVersionHasAuthor {
            id: package_version.id.clone(),
            version: package_version.version.clone(),
            author_id: author.id.clone(),
        };
        err!(insert(&this).into(packageversion_has_author::table).get_result(connection))
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, package_version: &PackageVersion, author: &Author) -> BackendResult<Self>
    {
        err!(packageversion_has_author::table.filter(
            packageversion_has_author::id.eq(&package_version.id)
            .and(packageversion_has_author::version.eq(&package_version.version))
            .and(packageversion_has_author::author_id.eq(&author.id))
        ).first(connection))
    }

    pub fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(packageversion_has_author::table.filter(
            packageversion_has_author::id.eq(self.id)
            .and(packageversion_has_author::version.eq(self.version))
            .and(packageversion_has_author::author_id.eq(self.author_id))
        )).execute(connection))
    }
}
