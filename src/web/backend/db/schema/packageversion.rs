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

use crypto::digest::Digest;
use crypto::sha2::Sha256;

use chrono::prelude::*;
use diesel::prelude::*;
use diesel::pg::Pg;
use diesel::{insert, update, delete};

use semver::{Version, VersionReq};

use treexml::Document as XmlDocument;
use treexml::Element;

use zip::{ZipArchive, ZipWriter, CompressionMethod};
use zip::result::ZipError;

use std::cmp::Ordering;
use std::iter::FlatMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::fs;
use std::str::FromStr;
use std::vec::IntoIter;

use ::web::backend::version::*;
use ::web::backend::xml::*;
use ::utils::error::{BackendError, BackendResult};
use ::web::backend::db::{dependency, package, packageversion, author, packageversion_has_author, packageversion_has_dependency};
use ::web::backend::db::schema::{Package, Tag, User, Dependency, Author, PackageVersionHasAuthor, PackageVersionHasDependency};
use ::web::backend::Storage;


#[derive(Queryable, Identifiable, AsChangeset, Insertable, Debug)]
#[table_name = "packageversion"]
#[changeset_options(treat_none_as_null="true")]
pub struct PackageVersion
{
    pub id: String,
    pub version: String,
    creation_date: NaiveDateTime,
    pub title: Option<String>,
    pub summary: Option<String>,
    updated: NaiveDateTime,
    pub description: Option<String>,
    version_download_count: i64,
    pub release_notes: Option<String>,
    hash: Option<String>,
    hash_algorithm: Option<String>,
    size: i64,
    pub icon_url: Option<String>,
}

impl PartialEq for PackageVersion
{
    fn eq(&self, other: &Self) -> bool
    {
        self.id == other.id && self.version == other.version
    }
}
impl Eq for PackageVersion {}

impl PartialOrd for PackageVersion
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(match self.id.cmp(&other.id) {
            Ordering::Equal => Version::parse(&self.version).unwrap().cmp(&Version::parse(&other.version).unwrap()),
            x => x,
        })
    }
}
impl Ord for PackageVersion
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        self.partial_cmp(other).unwrap()
    }
}

impl PackageVersion
{
    pub fn new<R: Read + Seek, C: Connection<Backend=Pg>>(
                        connection: &C,
                        user: &User,
                        storage: &Storage,
                        mut file: R,
                    ) -> BackendResult<Self>
    {
        let mut hasher = Sha256::new();
        let mut buffer = Vec::new();
        try!(file.read_to_end(&mut buffer));
        try!(file.seek(io::SeekFrom::Start(0)));
        hasher.input(&buffer);
        let hash = hasher.result_str();

        let mut zip: ZipArchive<R> = try!(ZipArchive::new(file));
        let nuspec = {
            let mut nuspec = None;
            for i in 0..zip.len()
            {
                let file = zip.by_index(i).unwrap();
                if file.name().contains(".nuspec") {
                    let doc: XmlDocument = try!(XmlDocument::parse(file)).clone();
                    nuspec = Some(try!(doc.root.ok_or(BackendError::InvalidXml("Empty Xml".into()))));
                }
            }
            match nuspec {
                Some(nuspec) => nuspec,
                None => return err!(Err(ZipError::FileNotFound)),
            }
        };

        let metadata = try!(nuspec.find_child(|entry| entry.name == "metadata").cloned().ok_or(BackendError::InvalidXml("Xml does not contain \"metadata\" tag".into())));

        let id = try!(try!(metadata.find_child(|tag| tag.name == "id").cloned().ok_or(BackendError::InvalidXml("Xml does not contain \"id\" tag".into()))).text.ok_or(BackendError::InvalidXml("\"id\" tag is empty".into())));
        let version_str = try!(try!(metadata.find_child(|tag| tag.name == "version").cloned().ok_or(BackendError::InvalidXml("Xml does not contain \"version\" tag".into()))).text.ok_or(BackendError::InvalidXml("\"version\" tag is empty".into())));
        let version = try!(Version::best_efford_parse(&version_str));

        let mut this = PackageVersion {
            id: id.clone(),
            version: format!("{}", version),
            creation_date: Utc::now().naive_utc(),
            title: None,
            summary: None,
            updated: Utc::now().naive_utc(),
            description: None,
            version_download_count: 0,
            release_notes: None,
            hash: Some(hash),
            hash_algorithm: Some(String::from("Sha256")),
            size: buffer.len() as i64,
            icon_url: None,
        };
        try!(this.set_from_xml(&nuspec));

        match connection.transaction(|| {
            match PackageVersion::get(connection, &id, &version) {
                Ok(pkgver) => { try!(pkgver.delete(connection, &storage)); },
                Err(_) => {},
            };

            let package = match Package::get(connection, &id) {
                Ok(mut pkg) => {
                    if &try!(pkg.maintainer(connection)) == user {
                        if try!(pkg.versions(connection)).into_iter().all(|ver| this > ver) {
                            try!(pkg.set_from_xml(&nuspec));
                            try!(pkg.update(connection, storage))
                        } else {
                            pkg
                        }
                    } else {
                        return Err(BackendError::PermissionDenied)
                    }
                },
                Err(_) => {
                    let mut pkg = Package::new(id, user);
                    try!(pkg.set_from_xml(&nuspec));
                    try!(insert(&pkg).into(package::table).get_result(connection))
                },
            };

            let this = try!(insert(&this).into(packageversion::table).get_result(connection));

            let tags: Option<&Element> = metadata.find_child(|entry| entry.name == "tags");
            let authors: Option<&Element> = metadata.find_child(|entry| entry.name == "authors");
            let dependencies: Option<&Element> = metadata.find_child(|entry| entry.name == "dependencies");

            match dependencies {
                Some(dependencies) => for dependency in
                    {
                        let children = dependencies.children.clone();
                        children.clone().into_iter().filter(|entry: &Element| entry.name == "group")
                                    .flat_map(|entry: Element| entry.children.into_iter())
                                    .chain(
                                        children.into_iter().filter(|entry: &Element| entry.name == "dependency")
                                    ).filter(|dependency| dependency.attributes.get("id").map(|id| !id.starts_with("chocolatey-core")).unwrap_or(false))
                    } {
                        let dependency: Element = dependency;

                        let found_id = try!(dependency.attributes.get("id").ok_or(BackendError::InvalidXml("Invalid Dependency, \"id\" attribute is missing".into())));
                        let req = match dependency.attributes.get("version") {
                            Some(ver) => try!(VersionReq::convert(ver)),
                            None => VersionReq::any(),
                        };

                        match Dependency::get(connection, &*found_id, &req) {
                            Ok(dep) => try!(dep.connect(connection, &this)),
                            Err(_) => { try!(Dependency::new(connection, &this, &try!(Package::get(connection, &*found_id)), &req)); },
                        };
                    },
                None => {},
            }

            match tags.and_then(|x| x.text.as_ref()) {
                Some(ref text) => {
                    let text: &String = text;
                    for tag in text.split_whitespace() {
                        match Tag::get(connection, tag) {
                            Ok(tag) => try!(tag.connect(connection, &package)),
                            Err(_) => { Tag::new(connection, &package,  String::from(tag)).unwrap(); },
                        };
                    }
                },
                None => {},
            }

            match authors.and_then(|x| x.text.as_ref()) {
                Some(ref text) => {
                    let text: &String = text;
                    for author in text.split(',').map(|x| x.trim()) {
                        match Author::get(connection, author) {
                            Ok(author) => try!(author.connect(connection, &this)),
                            Err(_) => { Author::new(connection, &this, String::from(author)).unwrap(); },
                        };
                    }
                },
                None => {},
            }

            let mut file = zip.into_inner();
            try!(file.seek(SeekFrom::Start(0)));
            try!(storage.store(&this, file));

            Ok(this)
        }) {
            Ok(x) => Ok(x),
            Err(x) => {
                storage.delete(&this);
                Err(x)
            },
        }
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, id: &str, version: &Version) -> BackendResult<Self>
    {
        err!(packageversion::table.filter(packageversion::id.eq(id).and(packageversion::version.eq(format!("{}", version)))).first(connection))
    }

    pub fn all<C: Connection<Backend=Pg>>(connection: &C) -> BackendResult<Vec<Self>>
    {
        err!(packageversion::table.load(connection))
    }

    pub fn update<C: Connection<Backend=Pg>>(&self, connection: &C, storage: &Storage) -> BackendResult<Self>
    {
        match connection.transaction(|| {
            let file = try!(storage.get(self));
            let mut archive = try!(ZipArchive::new(file));

            let stream = io::Cursor::new(Vec::new());
            let mut new_archive = ZipWriter::new(stream);

            for i in 0..archive.len()
            {
                let mut file = try!(archive.by_index(i));
                try!(new_archive.start_file(file.name(), CompressionMethod::Stored)); //TODO configurable?
                try!(io::copy(&mut file, &mut new_archive));
            }
            let mut stream = try!(new_archive.finish());
            try!(stream.seek(io::SeekFrom::Start(0)));

            //if these fail we have a broken package, so delete it
            let mut file = match storage.rewrite(self, archive.into_inner()) {
                Ok(x) => x,
                Err(_) => return Err(BackendError::CriticalUpdateFailure("Unable to rewrite the package".into())),
            };
            match io::copy(&mut stream, &mut file) {
                Ok(x) => x,
                Err(_) => return Err(BackendError::CriticalUpdateFailure("Unable to write to the rewriten package".into())),
            };
            match file.sync_all() {
                Ok(x) => x,
                Err(_) => return Err(BackendError::CriticalUpdateFailure("Unable to sync all changes".into())),
            };

            err!(update(packageversion::table.filter(
                    packageversion::id.eq(&self.id)
                    .and(packageversion::version.eq(&self.version))
                )).set(self).get_result(connection))
        }) {
            Ok(x) => Ok(x),
            x @ Err(BackendError::CriticalUpdateFailure(_)) => {
                try!(self.delete(connection, storage));
                x
            },
            Err(x) => Err(x),
        }
    }

    pub fn delete<C: Connection<Backend=Pg>>(&self, connection: &C, storage: &Storage) -> BackendResult<()>
    {
        connection.transaction(|| {
            {
                let blocking_dependencies = try!(self.blocking_dependencies(connection));
                if blocking_dependencies.len() > 0 {
                    let mut blocking_deps_str = String::new();

                    for (i, dep) in blocking_dependencies.into_iter().enumerate() {
                        for (j, pkg) in try!(dep.belongs(connection)).into_iter().enumerate() {
                            if i == 0  && j == 0 {
                                blocking_deps_str.push_str(&format!("\"{}\"", pkg.id()));
                            } else {
                                blocking_deps_str.push_str(&format!(", \"{}\"", pkg.id()));
                            }
                        }
                    }
                    blocking_deps_str.push_str(" are/is strictly depending on \"");
                    blocking_deps_str.push_str(self.id());
                    blocking_deps_str.push_str("\". Deletion is not possible.");
                    return Err(BackendError::BlockingDependency(blocking_deps_str.into()));
                }
            }
            for author in try!(self.authors(connection)) {
                try!(author.disconnect(connection, &self));
            }
            for dependency in try!(self.dependencies(connection)) {
                try!(dependency.disconnect(connection, &self));
            }
            let pkg = try!(self.package(&*connection));
            try!(delete(packageversion::table.filter(
                packageversion::id.eq(&self.id)
                .and(packageversion::version.eq(&self.version))
            )).execute(connection));
            if try!(pkg.versions(&*connection)).len() == 0 {
                try!(pkg.delete(&*connection));
            }
            storage.delete(&self);
            Ok(())
        })
    }

    pub fn package<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Package>
    {
        Package::get(connection, &self.id)
    }

    pub fn authors<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Author>>
    {
        let packageversion_has_authors: Vec<PackageVersionHasAuthor> = try!(packageversion_has_author::table.filter({
            packageversion_has_author::id.eq(&self.id)
            .and(packageversion_has_author::version.eq(&self.version))
        }).load(connection));

        let mut iterators = Vec::new();

        for package_has_author_entry in packageversion_has_authors.into_iter()
        {
            iterators.push(try!(author::table.filter(
                author::id.eq(package_has_author_entry.author_id)
            ).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    pub fn dependencies<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Dependency>>
    {
        let packageversion_has_dependencies: Vec<PackageVersionHasDependency> = try!(packageversion_has_dependency::table.filter({
            packageversion_has_dependency::id.eq(&self.id)
            .and(packageversion_has_dependency::version.eq(&self.version))
        }).load(connection));

        let mut iterators = Vec::new();

        for packageversion_has_dependency_entry in packageversion_has_dependencies.into_iter()
        {
            iterators.push(try!(dependency::table.filter({
                dependency::id.eq(&packageversion_has_dependency_entry.dependency_package_id).and(
                dependency::version_req.eq(&packageversion_has_dependency_entry.version_req))
            }).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(|entry| entry).collect())
    }

    //horror return type for performance reasons
    fn internal_dependencies_on_self<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<FlatMap<IntoIter<Vec<Dependency>>, IntoIter<Dependency>, fn(Vec<Dependency>) -> IntoIter<Dependency>>>
    {
        let packageversion_has_dependencies: Vec<PackageVersionHasDependency> = try!(packageversion_has_dependency::table.filter({
            packageversion_has_dependency::dependency_package_id.eq(&self.id)
        }).load(connection));

        let mut iterators = Vec::new();

        for packageversion_has_dependency_entry in packageversion_has_dependencies.into_iter()
        {
            iterators.push(try!(dependency::table.filter({
                dependency::id.eq(&packageversion_has_dependency_entry.dependency_package_id).and(
                dependency::version_req.eq(&packageversion_has_dependency_entry.version_req))
            }).load(connection)));
        }

        Ok(iterators.into_iter().flat_map(Vec::into_iter))
    }

    pub fn currently_dependending_package_versions<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Dependency>>
    {
        //manual filter to handle errors

        let mut results = Vec::new();

        for entry in try!(self.internal_dependencies_on_self(connection)) {
            if &try!(entry.newest_resolution(connection)) == self {
                results.push(entry);
            }
        }

        Ok(results)
    }

    pub fn possible_dependending_package_versions<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Dependency>>
    {
        //manual filter to handle errors

        let mut results = Vec::new();

        for entry in try!(self.internal_dependencies_on_self(connection)) {
            if try!(entry.possible_resolutions(connection)).contains(self) {
                results.push(entry);
            }
        }

        Ok(results)
    }

    pub fn blocking_dependencies<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Vec<Dependency>>
    {
        //manual filter to handle errors

        let mut results = Vec::new();

        for entry in try!(self.internal_dependencies_on_self(connection)) {
            let possible = try!(entry.possible_resolutions(connection));
            if possible.contains(self) && possible.len() == 1 {
                results.push(entry);
            }
        }

        Ok(results)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> Version
    {
        Version::from_str(&self.version).unwrap()
    }

    pub fn creation_date(&self) -> &NaiveDateTime
    {
        &self.creation_date
    }

    pub fn last_updated(&self) -> &NaiveDateTime
    {
        &self.updated
    }

    pub fn version_download_count(&self) -> i64
    {
        self.version_download_count
    }

    pub fn hash<'a>(&'a self) -> Option<&String>
    {
        self.hash.as_ref()
    }

    pub fn hash_algorithm(&self) -> Option<&String>
    {
        self.hash_algorithm.as_ref()
    }

    pub fn byte_size(&self) -> i64
    {
        self.size
    }

    pub fn download(&self, storage: &Storage) -> BackendResult<fs::File>
    {
        err!(storage.get(self))
    }
}
