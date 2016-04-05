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
#[insertable_into(packageversion_has_dependency)]
#[changeset_for(packageversion_has_dependency)]
struct PackageVersionHasDependency
{
    id: String,
    dependency_package_id: String,
    version: String,
    version_req: String,
}

impl PackageVersionHasDependency
{
    fn new<C: Connection<Backend=Pg>>(connection: &C, package_version: &PackageVersion, dependency: &Dependency) -> BackendResult<Self>
    {
        let this = PackageVersionHasDependency {
            id: package_version.id.clone(),
            dependency_package_id: dependency.id.clone(),
            version: package_version.version.clone(),
            version_req: dependency.version_req.clone(),
        };
        err!(insert(&this).into(packageversion_has_dependency::table).get_result(connection))
    }

    fn get<C: Connection<Backend=Pg>>(connection: &C, package_version: &PackageVersion, dependency: &Dependency) -> BackendResult<Self>
    {
        err!(packageversion_has_dependency::table.filter(
                packageversion_has_dependency::id.eq(&package_version.id)
                .and(packageversion_has_dependency::dependency_package_id.eq(&dependency.id))
                .and(packageversion_has_dependency::version.eq(&package_version.version))
                .and(packageversion_has_dependency::version_req.eq(&dependency.version_req))
            ).first(connection))
    }

    fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(packageversion_has_dependency::table.filter(
            packageversion_has_dependency::id.eq(self.id)
            .and(packageversion_has_dependency::dependency_package_id.eq(self.dependency_package_id))
            .and(packageversion_has_dependency::version.eq(self.version))
            .and(packageversion_has_dependency::version_req.eq(self.version_req)))
        ).execute(connection))
    }
}
