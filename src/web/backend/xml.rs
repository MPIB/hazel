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

use treexml::Element;

use diesel::prelude::*;
use diesel::pg::Pg;

use super::db::{Package, PackageVersion,Dependency};
use super::version::NugetToSemver;
use ::utils::error::{BackendResult, XmlError};

pub trait FromNugetXml
{
    fn set_from_xml(&mut self, elem: &Element) -> Result<(), XmlError>;
}

pub trait ToNugetXml
{
    fn xml_description<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Element>;
}

pub trait ToNugetFeedXml
{
    fn xml_entry<C: Connection<Backend=Pg>>(&self, base_url: &str, connection: &C) -> BackendResult<Element>;
}

impl FromNugetXml for PackageVersion
{
    fn set_from_xml(&mut self, package: &Element) -> Result<(), XmlError>
    {
        let metadata = try!(package.find_child(|tag| tag.name == "metadata").ok_or("Xml does not contain \"metadata\" tag"));
        self.title = metadata.find_child(|tag| tag.name == "title").and_then(|tag| tag.text.clone());
        self.summary = metadata.find_child(|tag| tag.name == "summary").and_then(|tag| tag.text.clone());
        self.description = metadata.find_child(|tag| tag.name == "description").and_then(|tag| tag.text.clone());
        self.release_notes = metadata.find_child(|tag| tag.name == "releaseNotes").and_then(|tag| tag.text.clone());
        self.icon_url = metadata.find_child(|tag| tag.name == "iconUrl").and_then(|tag| tag.text.clone());
        Ok(())
    }
}

impl FromNugetXml for Package
{
    fn set_from_xml(&mut self, package: &Element) -> Result<(), XmlError>
    {
        let metadata = try!(package.find_child(|tag| tag.name == "metadata").ok_or("Xml does not contain \"metadata\" tag"));
        self.project_url = metadata.find_child(|tag| tag.name == "projectUrl").and_then(|tag| tag.text.clone());
        self.license_url = metadata.find_child(|tag| tag.name == "licenseUrl").and_then(|tag| tag.text.clone());
        self.license_acceptance = match metadata.find_child(|tag| tag.name == "requireLicenseAcceptance").and_then(|tag| tag.text.clone()).as_ref().map(String::as_ref) {
            Some("true") => true,
            _ => false,
        };
        self.project_source_url = metadata.find_child(|tag| tag.name == "projectSourceUrl").and_then(|tag| tag.text.clone());
        self.package_source_url = metadata.find_child(|tag| tag.name == "packageSourceUrl").and_then(|tag| tag.text.clone());
        self.docs_url = metadata.find_child(|tag| tag.name == "docsUrl").and_then(|tag| tag.text.clone());
        self.mailing_list_url = metadata.find_child(|tag| tag.name == "mailingListUrl").and_then(|tag| tag.text.clone());
        self.bug_tracker_url = metadata.find_child(|tag| tag.name == "bugTrackerUrl").and_then(|tag| tag.text.clone());
        self.report_abuse_url = metadata.find_child(|tag| tag.name == "reportAbuseUrl").and_then(|tag| tag.text.clone());
        Ok(())
    }
}

impl ToNugetXml for PackageVersion
{
    fn xml_description<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Element>
    {
        let mut root = Element::new("package");
        let mut metadata = Element::new("metadata");

        let mut id = Element::new("id");
        id.text = Some(String::from(self.id()));
        metadata.children.push(id);

        let mut version = Element::new("version");
        version.text = Some(format!("{}", self.version()));
        metadata.children.push(version);

        let mut title = Element::new("title");
        title.text = self.title.clone();
        metadata.children.push(title);

        let mut summary = Element::new("summary");
        summary.text = self.summary.clone();
        metadata.children.push(summary);

        let mut description = Element::new("description");
        description.text = self.description.clone();
        metadata.children.push(description);

        let mut release_notes = Element::new("releaseNotes");
        release_notes.text = self.release_notes.clone();
        metadata.children.push(release_notes);

        let package = try!(self.package(connection));

        let mut project_url = Element::new("projectUrl");
        project_url.text = package.project_url.clone();
        metadata.children.push(project_url);

        let mut license_url = Element::new("licenseUrl");
        license_url.text = package.license_url.clone();
        metadata.children.push(license_url);

        let mut project_source_url = Element::new("projectSourceUrl");
        project_source_url.text = package.project_source_url.clone();
        metadata.children.push(project_source_url);

        let mut package_source_url = Element::new("packageSourceUrl");
        package_source_url.text = package.package_source_url.clone();
        metadata.children.push(package_source_url);

        let mut docs_url = Element::new("docsUrl");
        docs_url.text = package.docs_url.clone();
        metadata.children.push(docs_url);

        let mut mailing_list_url = Element::new("mailingListUrl");
        mailing_list_url.text = package.mailing_list_url.clone();
        metadata.children.push(mailing_list_url);

        let mut bug_tracker_url = Element::new("bugTrackerUrl");
        bug_tracker_url.text = package.bug_tracker_url.clone();
        metadata.children.push(bug_tracker_url);

        let mut report_abuse_url = Element::new("reportAbuseUrl");
        report_abuse_url.text = package.report_abuse_url.clone();
        metadata.children.push(report_abuse_url);

        let mut tags = Element::new("tags");
        let mut tags_string = String::new();
        for tag in try!(package.tags(connection)) {
            tags_string.push_str(tag.tag());
            tags_string.push_str(" ");
        }
        let tags_string_len = tags_string.len();
        if !tags_string.is_empty() { tags_string.truncate(tags_string_len-1) };
        tags.text = Some(tags_string);
        metadata.children.push(tags);

        let mut authors = Element::new("authors");
        let mut authors_string = String::new();
        for author in try!(self.authors(connection)) {
            authors_string.push_str(author.name());
            authors_string.push_str(" ");
        }
        let authors_string_len = authors_string.len();
        if !authors_string.is_empty() { authors_string.truncate(authors_string_len-1) };
        authors.text = Some(authors_string);
        metadata.children.push(authors);

        let mut dependencies = Element::new("dependencies");
        for dependency in try!(self.dependencies(connection))
        {
            dependencies.children.push(try!(dependency.xml_description(connection)));
        }
        metadata.children.push(dependencies);

        root.children.push(metadata);
        Ok(root)
    }
}

impl ToNugetXml for Dependency
{
    fn xml_description<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Element>
    {
        let mut root = Element::new("dependency");
        root.attributes.insert(String::from("id"), String::from(try!(self.requirement(connection)).id()));

        match try!(self.version_req().to_nuget()) {
            Some(x) => { root.attributes.insert(String::from("version"), x); },
            None => {},
        }

        Ok(root)
    }
}

impl ToNugetFeedXml for PackageVersion
{
    fn xml_entry<C: Connection<Backend=Pg>>(&self, base_url: &str, connection: &C) -> BackendResult<Element>
    {
        let package = try!(self.package(connection));
        let versions = try!(package.versions(connection));

        let mut root = Element::new("entry");
        let mut id = Element::new("id");
        id.text = Some(format!("{}/api/v2/Packages(Id=\'{}\',Version=\'{}\')", base_url, self.id(), self.version()));
        root.children.push(id);

        let mut title = Element::new("title");
        title.attributes.insert(String::from("type"), String::from("text"));
        title.text = Some(self.id().to_string());
        root.children.push(title);

        let mut summary = Element::new("summary");
        summary.attributes.insert(String::from("type"), String::from("text"));
        summary.text = self.summary.clone();
        root.children.push(summary);

        let mut updated = Element::new("updated");
        updated.text = Some(format!("{:?}", self.last_updated()));
        root.children.push(updated);

        let mut authors = Element::new("author");
        for author in try!(self.authors(connection))
        {
            let mut author_elem = Element::new("name");
            author_elem.text = Some(String::from(author.name()));
            authors.children.push(author_elem);
        }
        root.children.push(authors);

        let mut category = Element::new("category");
        category.attributes.insert(String::from("term"), String::from("NuGetGallery.V2FeedPackage"));
        category.attributes.insert(String::from("scheme"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices/scheme"));
        root.children.push(category);

        //<content type="application/zip" src="https://chocolatey.org/api/v2/package/1password/1.0.9.330" />
        let mut content = Element::new("content");
        content.attributes.insert(String::from("type"), String::from("application/zip"));
        content.attributes.insert(String::from("src"), format!("{}/api/v2/package/{}/{}", base_url, self.id(), self.version()));
        root.children.push(content);

        {

            let mut properties = Element::new("m:properties");
            properties.attributes.insert(String::from("xmlns:m"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices/metadata"));
            properties.attributes.insert(String::from("xmlns:d"), String::from("http://schemas.microsoft.com/ado/2007/08/dataservices"));

            let mut version = Element::new("d:Version");
            version.text = Some(format!("{}", self.version()));
            properties.children.push(version);

            let mut title = Element::new("d:Title");
            title.text = self.title.clone();
            properties.children.push(title);

            let mut description = Element::new("d:Description");
            description.text = self.description.clone();
            if description.text.is_none() {
                description.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(description);

            let mut tags = Element::new("d:Tags");
            tags.attributes.insert(String::from("xml:space"), String::from("preserve"));
            let mut tags_string = String::from(" ");
            for tag in try!(package.tags(connection)) {
                tags_string.push_str(tag.tag());
                tags_string.push_str(" ");
            }
            tags.text = Some(tags_string);
            properties.children.push(tags);

            let mut created = Element::new("d:Created");
            created.attributes.insert(String::from("m:type"), String::from("Edm.DateTime"));
            created.text = Some(format!("{:?}", self.creation_date()));
            properties.children.push(created);

            let mut dependencies = Element::new("d:Dependencies");
            let mut dependencies_string = String::from("");
            for dependency in try!(self.dependencies(connection))
            {
                dependencies_string.push_str(&format!("{}:{}:|",
                    try!(dependency.requirement(connection)).id(),
                    match try!(dependency.version_req().to_nuget()) {
                        Some(x) => { x },
                        None => { String::new() },
                    }
                ));
            }
            let dependencies_string_len = dependencies_string.len();
            if !dependencies_string.is_empty() { dependencies_string.truncate(dependencies_string_len-1) };
            dependencies.text = Some(dependencies_string);
            properties.children.push(dependencies);

            let mut download_count = Element::new("d:DownloadCount");
            download_count.attributes.insert(String::from("m:type"), String::from("Edm.Int32"));
            download_count.text = Some(format!("{}", versions.iter().map(|pkgver| { pkgver.version_download_count() }).fold(0, |total_count, count| { total_count + count })));
            properties.children.push(download_count);

            let mut version_download_count = Element::new("d:VersionDownloadCount");
            version_download_count.attributes.insert(String::from("m:type"), String::from("Edm.Int32"));
            version_download_count.text = Some(format!("{}", self.version_download_count()));
            properties.children.push(version_download_count);

            //TODO see metadata.rs
            //<d:GalleryDetailsUrl>https://chocolatey.org/packages/7-taskbar-tweaker/4.5.6</d:GalleryDetailsUrl>

            let mut report_abuse_url = Element::new("d:ReportAbuseUrl");
            report_abuse_url.text = package.report_abuse_url;
            if report_abuse_url.text.is_none() {
                report_abuse_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(report_abuse_url);

            let mut icon_url = Element::new("d:IconUrl");
            icon_url.text = self.icon_url.clone();
            if icon_url.text.is_none() {
                icon_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(icon_url);

            let mut is_latest_version = Element::new("d:IsLatestVersion");
            is_latest_version.attributes.insert(String::from("m:type"), String::from("Edm.Boolean"));
            is_latest_version.text = Some(String::from(
                match versions.iter()
                    .filter(|pkgver| !pkgver.version().is_prerelease())
                    .max_by_key(|pkgver| pkgver.version()).unwrap() == self {
                        true => "true",
                        false => "false",
                }
            ));
            properties.children.push(is_latest_version);

            let mut is_absolute_latest_version = Element::new("d:IsAbsoluteLatestVersion");
            is_absolute_latest_version.attributes.insert(String::from("m:type"), String::from("Edm.Boolean"));
            is_absolute_latest_version.text = Some(String::from(
                match versions.iter()
                    .max_by_key(|pkgver| pkgver.version()).unwrap() == self {
                        true => "true",
                        false => "false",
                }
            ));
            properties.children.push(is_absolute_latest_version);

            let mut is_prerelease = Element::new("d:IsPrerelease");
            is_prerelease.attributes.insert(String::from("m:type"), String::from("Edm.Boolean"));
            is_absolute_latest_version.text = Some(String::from(
                match self.version().is_prerelease() {
                    true => "true",
                    false => "false",
                }
            ));
            properties.children.push(is_prerelease);

            //TODO see metadata.rs
            //<d:Language m:null="true"></d:Language>

            let mut published = Element::new("d:Published");
            published.attributes.insert(String::from("m:type"), String::from("Edm.DateTime"));
            published.text = Some(format!("{:?}", self.creation_date()));
            properties.children.push(published);

            let mut license_url = Element::new("d:LicenseUrl");
            license_url.text = package.license_url;
            if license_url.text.is_none() {
                license_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(license_url);

            let mut require_license_acceptance = Element::new("d:RequireLicenseAcceptance");
            require_license_acceptance.attributes.insert(String::from("m:type"), String::from("Edm.Boolean"));
            require_license_acceptance.text = Some(package.license_acceptance.to_string());
            properties.children.push(require_license_acceptance);

            let mut package_hash = Element::new("d:PackageHash");
            package_hash.text = self.hash().map(|x| x.clone());
            properties.children.push(package_hash);

            let mut package_hash_algorithm = Element::new("d:PackageHashAlgorithm");
            package_hash_algorithm.text = self.hash_algorithm().map(|x| x.clone());
            properties.children.push(package_hash_algorithm);

            let mut package_size = Element::new("d:PackageSize");
            package_size.attributes.insert(String::from("m:type"), String::from("Edm.Int64"));
            package_size.text = Some(format!("{}", self.byte_size()));
            properties.children.push(package_size);

            let mut project_url = Element::new("d:ProjectUrl");
            project_url.text = package.project_url;
            if project_url.text.is_none() {
                project_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(project_url);

            let mut release_notes = Element::new("d:ReleaseNotes");
            release_notes.text = self.release_notes.clone();
            if release_notes.text.is_none() {
                release_notes.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(release_notes);

            let mut project_source_url = Element::new("d:ProjectSourceUrl");
            project_source_url.text = package.project_source_url;
            if project_source_url.text.is_none() {
                project_source_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(project_source_url);

            let mut package_source_url = Element::new("d:PackageSourceUrl");
            package_source_url.text = package.package_source_url;
            if package_source_url.text.is_none() {
                package_source_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(package_source_url);

            let mut docs_url = Element::new("d:DocsUrl");
            docs_url.text = package.docs_url;
            if docs_url.text.is_none() {
                docs_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(docs_url);

            let mut mailing_list_url = Element::new("d:MailingListUrl");
            mailing_list_url.text = package.mailing_list_url;
            if mailing_list_url.text.is_none() {
                mailing_list_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(mailing_list_url);

            let mut bug_tracker_url = Element::new("d:BugTrackerUrl");
            bug_tracker_url.text = package.bug_tracker_url;
            if bug_tracker_url.text.is_none() {
                bug_tracker_url.attributes.insert(String::from("m:null"), String::from("true"));
            }
            properties.children.push(bug_tracker_url);

            root.children.push(properties);
        }

        Ok(root)
    }
}
