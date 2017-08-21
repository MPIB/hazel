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

use semver::VersionReq;

use std::str::FromStr;

use ::utils::error::BackendResult;
use ::web::backend::db::{dependency, packageversion, packageversion_has_dependency};
use ::web::backend::db::schema::{Package, PackageVersion, PackageVersionHasDependency};

#[derive(Queryable, Debug, PartialEq, Eq, Insertable, Identifiable, AsChangeset)]
#[table_name="dependency"]
pub struct Dependency
{
    pub id: String,
    pub version_req: String,
}

impl Dependency
{
    pub fn new<C: Connection<Backend=Pg>>(connection: &C,
                package_version: &PackageVersion,
                package: &Package,
                version_req: &VersionReq,
           ) -> BackendResult<Self>
    {
        let this = Dependency {
            id: String::from(package.id()),
            version_req: format!("{}", version_req),
        };

        let dep = try!(insert(&this).into(dependency::table).get_result(connection));
        try!(PackageVersionHasDependency::new(connection, package_version, &this));
        Ok(dep)
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, id: &str, version_req: &VersionReq) -> BackendResult<Self>
    {
        err!(dependency::table.filter(
                dependency::id.eq(id)
                .and(dependency::version_req.eq(format!("{}", version_req)))
            ).first(connection))
    }

    pub fn belongs<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<PackageVersion>>
    {
        let packageversion_has_dependencies: Vec<PackageVersionHasDependency> = try!(packageversion_has_dependency::table.filter({
            packageversion_has_dependency::dependency_package_id.eq(&self.id)
            .and(packageversion_has_dependency::version_req.eq(&self.version_req))
        }).load(connection));

        let mut iterators = Vec::new();

        for packageversion_has_dependency_entry in packageversion_has_dependencies.into_iter()
        {
            iterators.push(try!(packageversion::table.filter(
                packageversion::id.eq(packageversion_has_dependency_entry.id)
                .and(packageversion::version.eq(packageversion_has_dependency_entry.version))
            ).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    pub fn connect<C: Connection<Backend=Pg>>(&self, connection: &C, version: &PackageVersion) -> BackendResult<()>
    {
        match PackageVersionHasDependency::get(connection, &version, &self) {
            Ok(_) => Ok(()),
            Err(_) => match PackageVersionHasDependency::new(connection, &version, &self) {
                Ok(_) => Ok(()),
                Err(x) => Err(x),
            },
        }
    }

    pub fn disconnect<C: Connection<Backend=Pg>>(self, connection: &C, version: &PackageVersion) -> BackendResult<()>
    {
        match PackageVersionHasDependency::get(connection, &version, &self) {
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
        err_discard!(delete(dependency::table.filter(
            dependency::id.eq(self.id)
            .and(dependency::version_req.eq(self.version_req)))
        ).execute(connection))
    }

    pub fn requirement<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Package>
    {
        Package::get(connection, &self.id)
    }

    pub fn version_req(&self) -> VersionReq
    {
        VersionReq::from_str(&self.version_req).unwrap()
    }

    pub fn possible_resolutions<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<PackageVersion>>
    {
        let package = try!(Package::get(connection, &self.id));
        Ok(try!(package.versions(connection)).into_iter().filter(|package_version: &PackageVersion| {
            self.version_req().matches(&package_version.version())
        }).collect())
    }

    pub fn newest_resolution<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<PackageVersion>
    {
        Ok(try!(self.possible_resolutions(connection)).into_iter().max_by_key(|x| x.version()).expect("Cannot compare version?"))
    }
}
