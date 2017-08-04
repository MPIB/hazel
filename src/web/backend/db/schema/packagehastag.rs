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
use ::web::backend::db::package_has_tag;
use ::web::backend::db::schema::{Package, Tag};

#[derive(Queryable, Debug, Insertable, Identifiable, AsChangeset)]
#[table_name="package_has_tag"]
pub struct PackageHasTag
{
    pub id: String,
    pub package_id: String,
}

impl PackageHasTag
{
    pub fn new<C: Connection<Backend=Pg>>(connection: &C, package: &Package, tag: &Tag) -> BackendResult<Self>
    {
        let this = PackageHasTag {
            id: tag.id.clone(),
            package_id: package.id.clone(),
        };
        err!(insert(&this).into(package_has_tag::table).get_result(connection))
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, package: &Package, tag: &Tag) -> BackendResult<Self>
    {
        err!(package_has_tag::table.filter(
                package_has_tag::id.eq(&tag.id)
                .and(package_has_tag::package_id.eq(&package.id))
            ).first(connection))
    }

    pub fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(package_has_tag::table.filter(
            package_has_tag::id.eq(self.id)
            .and(package_has_tag::package_id.eq(self.package_id))
        )).execute(connection))
    }
}
