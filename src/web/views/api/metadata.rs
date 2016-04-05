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

use iron::{Request, Response, IronResult};
use iron::status;
use iron::mime::Mime;

pub fn metadata(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, format!(

//TODO <Property Name=\"GalleryDetailsUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
//TODO<Property Name=\"Language\" Type=\"Edm.String\" Nullable=\"true\"/>

"<edmx:Edmx xmlns:edmx=\"http://schemas.microsoft.com/ado/2007/06/edmx\" Version=\"1.0\">
    <edmx:DataServices xmlns:m=\"http://schemas.microsoft.com/ado/2007/08/dataservices/metadata\" m:DataServiceVersion=\"2.0\">
        <Schema xmlns:d=\"http://schemas.microsoft.com/ado/2007/08/dataservices\" xmlns:m=\"http://schemas.microsoft.com/ado/2007/08/dataservices/metadata\" xmlns=\"http://schemas.microsoft.com/ado/2006/04/edm\" Namespace=\"NuGetGallery\">
            <EntityType Name=\"V2FeedPackage\" m:HasStream=\"true\">
                <Key>
                    <PropertyRef Name=\"Id\"/>
                    <PropertyRef Name=\"Version\"/>
                </Key>
                <Property Name=\"Id\" Type=\"Edm.String\" Nullable=\"false\" m:FC_TargetPath=\"SyndicationTitle\" m:FC_ContentKind=\"text\" m:FC_KeepInContent=\"false\"/>
                <Property Name=\"Version\" Type=\"Edm.String\" Nullable=\"false\"/>
                <Property Name=\"Title\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"Summary\" Type=\"Edm.String\" Nullable=\"true\" m:FC_TargetPath=\"SyndicationSummary\" m:FC_ContentKind=\"text\" m:FC_KeepInContent=\"false\"/>
                <Property Name=\"Description\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"Tags\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"Authors\" Type=\"Edm.String\" Nullable=\"true\" m:FC_TargetPath=\"SyndicationAuthorName\" m:FC_ContentKind=\"text\" m:FC_KeepInContent=\"false\"/>
                <Property Name=\"Created\" Type=\"Edm.DateTime\" Nullable=\"false\"/>
                <Property Name=\"Dependencies\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"DownloadCount\" Type=\"Edm.Int32\" Nullable=\"false\"/>
                <Property Name=\"VersionDownloadCount\" Type=\"Edm.Int32\" Nullable=\"false\"/>
                <Property Name=\"ReportAbuseUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"IconUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"IsLatestVersion\" Type=\"Edm.Boolean\" Nullable=\"false\"/>
                <Property Name=\"IsAbsoluteLatestVersion\" Type=\"Edm.Boolean\" Nullable=\"false\"/>
                <Property Name=\"IsPrerelease\" Type=\"Edm.Boolean\" Nullable=\"false\"/>
                <Property Name=\"LastUpdated\" Type=\"Edm.DateTime\" Nullable=\"false\" m:FC_TargetPath=\"SyndicationUpdated\" m:FC_ContentKind=\"text\" m:FC_KeepInContent=\"false\"/>
                <Property Name=\"Published\" Type=\"Edm.DateTime\" Nullable=\"false\"/>
                <Property Name=\"LicenseUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"RequireLicenseAcceptance\" Type=\"Edm.Boolean\" Nullable=\"false\"/>
                <Property Name=\"PackageHash\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"PackageHashAlgorithm\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"PackageSize\" Type=\"Edm.Int64\" Nullable=\"false\"/>
                <Property Name=\"ProjectUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"ReleaseNotes\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"ProjectSourceUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"PackageSourceUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"DocsUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"MailingListUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
                <Property Name=\"BugTrackerUrl\" Type=\"Edm.String\" Nullable=\"true\"/>
            </EntityType>
            <EntityContainer Name=\"FeedContext_x0060_1\" m:IsDefaultEntityContainer=\"true\">
                <EntitySet Name=\"Packages\" EntityType=\"NuGetGallery.V2FeedPackage\"/>
                <FunctionImport Name=\"Search\" EntitySet=\"Packages\" ReturnType=\"Collection(NuGetGallery.V2FeedPackage)\" m:HttpMethod=\"GET\">
                    <Parameter Name=\"searchTerm\" Type=\"Edm.String\" Mode=\"In\"/>
                    <Parameter Name=\"targetFramework\" Type=\"Edm.String\" Mode=\"In\"/>
                    <Parameter Name=\"includePrerelease\" Type=\"Edm.Boolean\" Mode=\"In\"/>
                </FunctionImport>
                <FunctionImport Name=\"FindPackagesById\" EntitySet=\"Packages\" ReturnType=\"Collection(NuGetGallery.V2FeedPackage)\" m:HttpMethod=\"GET\">
                    <Parameter Name=\"id\" Type=\"Edm.String\" Mode=\"In\"/>
                </FunctionImport>
                <FunctionImport Name=\"GetUpdates\" EntitySet=\"Packages\" ReturnType=\"Collection(NuGetGallery.V2FeedPackage)\" m:HttpMethod=\"GET\">
                    <Parameter Name=\"packageIds\" Type=\"Edm.String\" Mode=\"In\"/>
                    <Parameter Name=\"versions\" Type=\"Edm.String\" Mode=\"In\"/>
                    <Parameter Name=\"includePrerelease\" Type=\"Edm.Boolean\" Mode=\"In\"/>
                    <Parameter Name=\"includeAllVersions\" Type=\"Edm.Boolean\" Mode=\"In\"/>
                    <Parameter Name=\"targetFrameworks\" Type=\"Edm.String\" Mode=\"In\"/>
                </FunctionImport>
            </EntityContainer>
        </Schema>
    </edmx:DataServices>
</edmx:Edmx>"
    ), {
        let mime: Mime = "application/atom+xml".parse().unwrap();
        mime
    })))
}
