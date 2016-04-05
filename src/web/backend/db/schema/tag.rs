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

#[derive(Queryable, Debug, PartialEq, Eq)]
#[insertable_into(tag)]
pub struct Tag
{
    id: String,
}

impl Tag
{
    pub fn new<C: Connection<Backend=Pg>>(connection: &C,
               package: &Package,
               name: String,
              ) -> BackendResult<Self>
    {
        let this = Tag {
            id: name
        };

        let result = try!(insert(&this).into(tag::table).get_result(connection));
        try!(PackageHasTag::new(connection, package, &this));
        Ok(result)
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, id: &str) -> BackendResult<Self>
    {
        err!(tag::table.filter(
            tag::id.eq(id)
        ).first(connection))
    }

    pub fn belongs<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Package>>
    {
        let package_has_tags: Vec<PackageHasTag> = try!(package_has_tag::table.filter({
                package_has_tag::id.eq(&self.id)
            }).load(connection));

        let mut iterators = Vec::new();

        for package_has_tag_entry in package_has_tags.into_iter()
        {
            iterators.push(try!(package::table.filter({
                package::id.eq(package_has_tag_entry.package_id)
            }).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    pub fn connect<C: Connection<Backend=Pg>>(&self, connection: &C, package: &Package) -> BackendResult<()>
    {
        err!(match PackageHasTag::get(connection, package, &self) {
            Ok(_) => Ok(()),
            Err(_) => match PackageHasTag::new(connection, package, self) {
                Ok(_) => Ok(()),
                Err(x) => Err(x),
            },
        })
    }

    pub fn disconnect<C: Connection<Backend=Pg>>(self, connection: &C, package: &Package) -> BackendResult<()>
    {
        match PackageHasTag::get(connection, package, &self) {
            Ok(has) => {
                match connection.transaction(move || {
                    try!(has.delete(connection));
                    if try!(self.belongs(connection)).len() == 0 { try!(self.delete(connection)); }
                    Ok(())
                }) {
                    Ok(()) => Ok(()),
                    Err(TransactionError::CouldntCreateTransaction(err)) => Err(BackendError::DBError(err)),
                    Err(TransactionError::UserReturnedError(err)) => Err(err),
                }
            },
            Err(x) => Err(x),
        }
    }

    pub fn delete<C: Connection<Backend=Pg>>(self, connection: &C) -> BackendResult<()>
    {
        err_discard!(delete(tag::table.filter(tag::id.eq(self.id))).execute(connection))
    }

    pub fn tag(&self) -> &str
    {
        &self.id
    }
}
