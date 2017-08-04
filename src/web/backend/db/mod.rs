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

mod tables {
    table! (dependency { id -> Text , version_req -> Text , });
    table! (packageversion_has_dependency {
            id -> Text , dependency_package_id -> Text , version -> Text ,
            version_req -> Text , });
    table! (author { id -> Text , });
    table! (packageversion {
            id -> Text , version -> Text , creation_date -> Timestamp , title
            -> Nullable<Text> , summary -> Nullable<Text> , updated ->
            Timestamp , description -> Nullable<Text> , version_download_count
            -> Int8 , release_notes -> Nullable<Text> , hash -> Nullable<Text>
            , hash_algorithm -> Nullable<Text> , size -> Int8 , icon_url ->
            Nullable<Text> , });
    table! (packageversion_has_author {
            id -> Text , version -> Text , author_id -> Text , });
    table! (tag { id -> Text , });
    table! (package {
            id -> Text , project_url -> Nullable<Text> , license_url ->
            Nullable<Text> , license_acceptance -> Bool, project_source_url -> Nullable<Text> ,
            package_source_url -> Nullable<Text> , docs_url -> Nullable<Text>
            , mailing_list_url -> Nullable<Text> , bug_tracker_url ->
            Nullable<Text> , report_abuse_url -> Nullable<Text> , maintainer -> Text , });
    table! (package_has_tag { id -> Text , package_id -> Text , });
    table! (hazeluser { id -> Text , name -> Text , mail -> Nullable<Text>, mail_key -> Nullable<Text>, confirmed -> Bool, provider -> Text ,
            password -> Nullable<Text> , apikey -> Nullable<Text> , });
}

mod schema;
pub use self::schema::*;

use self::tables::package;
use self::tables::packageversion;
use self::tables::dependency;
use self::tables::packageversion_has_dependency;
use self::tables::author;
use self::tables::packageversion_has_author;
use self::tables::tag;
use self::tables::package_has_tag;
use self::tables::hazeluser;
