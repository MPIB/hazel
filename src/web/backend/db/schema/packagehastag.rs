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

#[derive(Queryable, Debug)]
#[insertable_into(package_has_tag)]
#[changeset_for(package_has_tag)]
struct PackageHasTag
{
    id: String,
    package_id: String,
}

impl PackageHasTag
{
    fn new<C: Connection<Backend=Pg>>(connection: &C, package: &Package, tag: &Tag) -> BackendResult<Self>
    {
        let this = PackageHasTag {
            id: tag.id.clone(),
            package_id: package.id.clone(),
        };
        err!(insert(&this).into(package_has_tag::table).get_result(connection))
    }

    fn get<C: Connection<Backend=Pg>>(connection: &C, package: &Package, tag: &Tag) -> BackendResult<Self>
    {
        err!(package_has_tag::table.filter(
                package_has_tag::id.eq(&tag.id)
                .and(package_has_tag::package_id.eq(&package.id))
            ).first(connection))
    }

    fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(package_has_tag::table.filter(
            package_has_tag::id.eq(self.id)
            .and(package_has_tag::package_id.eq(self.package_id))
        )).execute(connection))
    }
}
