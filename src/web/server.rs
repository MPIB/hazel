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

use iron::{Iron, Request, Response, IronResult};
use iron::middleware::Chain;
use iron::headers::UserAgent;
use iron::typemap::Key;
use iron::status;
use iron::modifiers::Redirect;
use hyper::server::Listening;
use hyper_native_tls::NativeTlsServer;
use mount::Mount;
use router::Router;
use persistent::{Read, Write};
use staticfile::Static;
use multipart::server::iron::Intercept;

use r2d2::Pool;
use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;

use super::backend::Storage;
use utils::middleware::Logger;
use utils::middleware::PathNormalizer;
use utils::middleware::{SessionManager, SessionInfo};

use std::collections::HashMap;
use std::path::PathBuf;

use ::utils::CONFIG;

use super::views::api::index::index;
use super::views::api::metadata::metadata;
use super::views::api::packages::packages;
use super::views::api::download::download;
use super::views::api::upload::upload;
use super::views::api::delete::delete;
use super::views::api::package::package;
use super::views::api::search::search;
use super::views::api::packagesbyid::packagesbyid;
use super::views::api::updates::updates;
use super::views::api::complete_ids::complete_ids;
use super::views::api::complete_ver::complete_ver;

use super::views::interface::index::index as interface_index;
use super::views::interface::user::user as interface_user;
use super::views::interface::user_update::update as interface_user_update;
use super::views::interface::register::register as interface_register;
use super::views::interface::login::login as interface_login;
use super::views::interface::logout::logout as interface_logout;
use super::views::interface::apikey::apikey as interface_apikey;
use super::views::interface::package::package_newestver as interface_package_newestver;
use super::views::interface::package::package as interface_package;
use super::views::interface::pkg_update::pkg_update as interface_pkg_update;
use super::views::interface::pkgver_update::pkgver_update as interface_pkgver_update;
use super::views::interface::transfer::transfer as interface_transfer;
use super::views::interface::mail_confirmation::mail_confirmation as interface_mail_confirmation;
use super::views::interface::mail_resend::mail_resend as interface_mail_resend;

#[derive(Copy, Clone)]
pub struct ConnectionPoolKey;
impl Key for ConnectionPoolKey { type Value = Pool<ConnectionManager<PgConnection>>; }

#[derive(Copy, Clone)]
pub struct StorageKey;
impl Key for StorageKey { type Value = Storage; }

#[derive(Copy, Clone)]
pub struct SessionStoreKey;
impl Key for SessionStoreKey { type Value = HashMap<String, SessionInfo>; }

pub fn start(pool: Pool<ConnectionManager<PgConnection>>, storage: Storage) -> Listening {
    let mut mount = Mount::new();

    // home
    {
        let mut interface = Router::new();

        interface.get("/", {
            fn redirect(req: &mut Request) -> IronResult<Response> {
                //this is such bullshit
                let user_agent = req.headers.get::<UserAgent>();
                if user_agent.is_some() && (user_agent.unwrap().0.contains("Chocolatey") || user_agent.unwrap().0.contains("NuGet")) {
                    Ok(Response::with(status::Ok))
                } else {
                    Ok(Response::with((status::TemporaryRedirect, Redirect({
                        let mut base = req.url.clone();
                        base.as_mut().path_segments_mut().unwrap().push("index");
                        base
                    }))))
                }
            }
            redirect
        }, "root_redirect");
        interface.get("/index", {
            fn redirect(req: &mut Request) -> IronResult<Response> {
                Ok(Response::with((status::TemporaryRedirect, Redirect({
                    let mut base = req.url.clone();
                    base.as_mut().path_segments_mut().unwrap().push("1");
                    base
                }))))
            }
            redirect
        }, "index_redirect");
        interface.get("/index/:page", interface_index, "index");
        interface.get("/user", interface_user, "user");
        interface.get("/user/edit", interface_user, "user_edit");
        interface.post("/user/edit", interface_user_update, "user_update");
        interface.post("/register", interface_register, "register");
        interface.post("/login", interface_login, "login");
        interface.get("/logout", interface_logout, "logout");
        interface.post("/apikey/reset", interface_apikey, "apikey_reset");
        interface.post("/apikey/revoke", interface_apikey, "apikey_revoke");
        interface.get("/packages/:id", interface_package_newestver, "package_by_id");
        interface.get("/packages/:id/:version", interface_package, "package_by_id_version");
        interface.get("/packages/:id/:version/edit", interface_package, "package_edit");
        interface.post("/packages/:id/edit", interface_pkg_update, "package_update");
        interface.post("/packages/:id/:version/edit", interface_pkgver_update, "packageversion_update");
        interface.get("/packages/transfer/:id/:new_maintainer", interface_transfer, "package_transfer");
        if CONFIG.auth.mail.is_some() {
            interface.post("/mail_confirmation/resend", interface_mail_resend, "mail_confirm_resend");
            interface.get("/mail_confirmation/:key", interface_mail_confirmation, "mail_confirm");
        }

        mount.mount("/css/", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("css")));
        mount.mount("/img/", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("img")));
        mount.mount("/js/", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("js")));

        mount.mount("/android-chrome-36x36.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-36x36.png")));
        mount.mount("/android-chrome-48x48.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-48x48.png")));
        mount.mount("/android-chrome-72x72.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-72x72.png")));
        mount.mount("/android-chrome-96x96.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-96x96.png")));
        mount.mount("/android-chrome-144x144.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-144x144.png")));
        mount.mount("/android-chrome-192x192.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/android-chrome-192x192.png")));
        mount.mount("/apple-touch-icon-57x57.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-57x57.png")));
        mount.mount("/apple-touch-icon-60x60.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-60x60.png")));
        mount.mount("/apple-touch-icon-72x72.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-72x72.png")));
        mount.mount("/apple-touch-icon-76x76.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-76x76.png")));
        mount.mount("/apple-touch-icon-114x114.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-114x114.png")));
        mount.mount("/apple-touch-icon-120x120.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-120x120.png")));
        mount.mount("/apple-touch-icon-144x144.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-144x144.png")));
        mount.mount("/apple-touch-icon-152x152.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-152x152.png")));
        mount.mount("/apple-touch-icon-180x180.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-180x180.png")));
        mount.mount("/apple-touch-icon-precomposed.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon-precomposed.png")));
        mount.mount("/apple-touch-icon.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/apple-touch-icon.png")));
        mount.mount("/browserconfig.xml", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/browserconfig.xml")));
        mount.mount("/favicon-16x16.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/favicon-16x16.png")));
        mount.mount("/favicon-32x32.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/favicon-32x32.png")));
        mount.mount("/favicon-96x96.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/favicon-96x96.png")));
        mount.mount("/favicon.ico", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/favicon.ico")));
        mount.mount("/manifest.json", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/manifest.json")));
        mount.mount("/mstile-70x70.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/mstile-70x70.png")));
        mount.mount("/mstile-144x144.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/mstile-144x144.png")));
        mount.mount("/mstile-150x150.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/mstile-150x150.png")));
        mount.mount("/mstile-310x150.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/mstile-310x150.png")));
        mount.mount("/mstile-310x310.png", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/mstile-310x310.png")));
        mount.mount("/safari-pinned-tab.svg", Static::new(PathBuf::from(CONFIG.web.resources.clone()).join("favicon/safari-pinned-tab.svg")));

        mount.mount("/", interface);
    }

    // api
    {
        let mut feed = Router::new();

        feed.get("", index, "index");
        feed.get("$metadata", metadata, "metadata");

        // get all package(s)
        feed.get("Packages()", packages, "packages_braces");
        feed.get("Packages", packages, "packages");

        // download specific package
        feed.get("package/:id/:version", download, "package_download");

        // add/delete package
        feed.post("package", upload, "package_upload_post");
        feed.put("package", upload, "package_upload_put");
        feed.delete("package/:id/:version", delete, "packageversion_delete");
        feed.delete("package/:id", delete, "package_delete");

        // functions aka filter packages
        feed.get("FindPackagesById()", packagesbyid, "find_package_by_id_braces");
        feed.get("FindPackagesById", packagesbyid, "find_package_by_id");
        feed.get("GetUpdates()", updates, "package_update_braces");
        feed.get("GetUpdates", updates, "package_update");
        feed.get("Search()", search, "search_braces");
        feed.get("Search", search, "search");

        // tab-completion
        feed.get("package-ids", complete_ids, "complete_ids");
        feed.get("package-versions/:id", complete_ver, "complete_versions");

        //Package(Id=':id',Version=':version')
        feed.get("*", package, "package_braces"); //Router does not handle this correctly

        mount.mount("/api/v2/", feed);
    }

    let mut chain = Chain::new(mount);
    chain.link_around(SessionManager);
    chain.link_before(PathNormalizer);
    chain.link_before(Logger);
    chain.link_before(Intercept::default().file_size_limit((CONFIG.web.max_upload_filesize_mb * 1024 * 1024) as u64));
    chain.link(Read::<ConnectionPoolKey>::both(pool));
    chain.link(Read::<StorageKey>::both(storage));
    chain.link(Write::<SessionStoreKey>::both(HashMap::new()));

    match CONFIG.server.https.clone() {
        Some(config) => Iron::new(chain).https(("0.0.0.0", CONFIG.server.port), NativeTlsServer::new(PathBuf::from(config.certificate), &config.key).unwrap()),
        None => Iron::new(chain).http(("0.0.0.0", CONFIG.server.port)),
    }.unwrap()
}
