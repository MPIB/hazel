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

use bcrypt;

use cldap::RustLDAP;
use cldap::codes::scopes::LDAP_SCOPE_SUBTREE;

use diesel::prelude::*;
use diesel::pg::Pg;
use diesel::{insert, update, delete};

use lettre::email::EmailBuilder;
use lettre::transport::EmailTransport;
use lettre::transport::smtp::*;
use lettre::transport::smtp::authentication::*;

use uuid::Uuid;

use std::ptr;

use ::utils::CONFIG;
use ::utils::error::{BackendError, BackendResult, MailError, LDAPError, LoginError};
use ::web::backend::db::hazeluser;
use ::web::backend::db::schema::Package;

pub enum Authentication {
    LDAP,
    //Plain(password: String)
    Plain(String)
}

#[derive(Queryable, Debug, Serialize, Identifiable, Insertable, AsChangeset)]
#[table_name = "hazeluser"]
pub struct User
{
    pub id: String,
    pub name: String,
    mail: Option<String>,
    mail_key: Option<String>,
    confirmed: bool,
    provider: String,
    password: Option<String>,
    apikey: Option<String>,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for User {}

impl User
{
    fn new<C: Connection<Backend=Pg>>(connection: &C, username: String, fullname: String, mail: Option<String>, authentication: Authentication, apikey: Option<String>) -> BackendResult<Self>
    {
        let (provider, password) = match authentication
        {
            Authentication::LDAP => (String::from("LDAP"), None),
            Authentication::Plain(password) => (String::from("Plain"), Some(password)),
        };

        let this = User {
            id: username,
            name: fullname,
            mail: mail.clone(),
            mail_key: if mail.is_some() { Some(Uuid::new_v4().simple().to_string()) } else { None },
            confirmed: if mail.is_some() { false } else { true },
            provider: provider,
            password: password,
            apikey: apikey
        };
        err!(insert(&this).into(hazeluser::table).get_result(connection))
    }

    pub fn get<C: Connection<Backend=Pg>>(connection: &C, username: &String) -> BackendResult<Self>
    {
        err!(hazeluser::table.filter(
                hazeluser::id.eq(username)
            ).first(connection))
    }

    pub fn mail(&self) -> Option<String>
    {
        self.mail.clone()
    }

    pub fn set_mail<C: Connection<Backend=Pg>>(&mut self, connection: &C, mail: String) -> BackendResult<Self>
    {
        match &*self.provider {
            "Plain" => {
                connection.transaction(|| {
                    self.mail = Some(mail);
                    self.confirmed = CONFIG.auth.mail.is_none();
                    self.mail_key = Some(Uuid::new_v4().simple().to_string());
                    self.apikey = None;
                    let update = try!(self.update(&*connection));
                    err!(match update.send_mail() {
                        Ok(()) => Ok(update),
                        Err(MailError::ConfigMissing) => Ok(update),
                        Err(x) => Err(x),
                    })
                })
            },
            _ => Err(BackendError::InvalidProviderForOP)
        }
    }

    pub fn confirmed(&self) -> bool
    {
        self.confirmed
    }

    pub fn set_confirmed<C: Connection<Backend=Pg>>(&mut self, connection: &C, state: bool) -> BackendResult<User>
    {
        self.confirmed = state;
        self.update(&*connection)
    }

    pub fn confirm_mail<C: Connection<Backend=Pg>>(connection: &C, key: String) -> BackendResult<User>
    {
        err!(hazeluser::table.filter(
                hazeluser::mail_key.eq(key)
            ).first(connection)).and_then(move |mut user: User| user.set_confirmed(&*connection, true))
    }

    pub fn send_mail(&self) -> Result<(), MailError>
    {
        match CONFIG.auth.mail.as_ref() {
            Some(ref config) => {
                let email = EmailBuilder::new()
                                    .to(&*match self.mail.as_ref() {
                                        Some(mail) => mail.clone(),
                                        None => return Err(MailError::UserHasNoMailAddress),
                                    })
                                    .from(&*config.mail_address)
                                    .alternative(&*format!(
                                                        "<html>\
                                                        <body>\
                                                        <h3>Welcome to {0}</h3><br>\
                                                        Please click the confirmation link below to activate your account.<br><br>\
                                                        <h2>{1}/mail_confirmation/{2}</h2></br>
                                                        Greetings
                                                        </body>\
                                                        </html>",
                                                    config.fullname_website, config.domain_website, self.mail_key.as_ref().unwrap()),
                                                &*format!(
                                                        "Hi,\n\
                                                        \n\
                                                        Welcome to {0}\n\
                                                        Please click the confirmation link below to activate your account.\n\
                                                        \n\
                                                        {1}/mail_confirmation/{2}\n\
                                                        \n\
                                                        Greetings",
                                                    config.fullname_website, config.domain_website, self.mail_key.as_ref().unwrap()))
                                    .subject(&*format!("[Confirmation] User Account on {}", config.fullname_website))
                                    .build()
                                    .unwrap();

                // Connect to a remote server on a custom port
                let mut mailer = SmtpTransportBuilder::new((&*config.hostname,
                config.port.unwrap_or(SUBMISSION_PORT))).unwrap()
                    // Set the name sent during EHLO/HELO, default is `localhost`
                    .hello_name(&*config.hello_name)
                    // Add credentials for authentication
                    .credentials(&*config.username.as_ref().unwrap_or(&config.mail_address), &*config.password)
                    // Specify a TLS security level.
                    .security_level(match config.encrypt {
                        Some(true) => SecurityLevel::AlwaysEncrypt,
                        Some(false) => SecurityLevel::NeverEncrypt,
                        None => SecurityLevel::Opportunistic,
                    })
                    // Enable SMTPUTF8 if the server supports it
                    .smtp_utf8(config.utf8)
                    // Configure expected authentication mechanism
                    .authentication_mechanism(match config.authentication.as_ref().map(String::as_ref) {
                        Some("Plain") => Mechanism::Plain,
                        Some("CramMd5") => Mechanism::CramMd5,
                        Some(_) => return Err(MailError::UnknownAuthenticationMechanism),
                        None => Mechanism::Plain,
                    })
                    // Enable connection reuse
                    .connection_reuse(false).build();

                try!(mailer.send(email));
                mailer.close();
                Ok(())
            },
            None => {
                Err(MailError::ConfigMissing)
            }
        }

    }

    pub fn apikey(&self) -> Option<String>
    {
        self.apikey.clone()
    }

    pub fn delete<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<()>
    {
        connection.transaction(|| {
            let admin = try!(User::get(connection, &String::from("admin")));
            for mut pkg in try!(Package::all(connection)).into_iter().filter(|package| package.maintainer == self.id) {
                try!(pkg.update_maintainer(connection, &admin));
            }
            err_discard!(delete(hazeluser::table.filter(hazeluser::id.eq(&self.id))).execute(connection))
        })
    }

    pub fn update<C: Connection<Backend=Pg>>(&self, connection: &C) -> BackendResult<Self>
    {
        err!(update(hazeluser::table.filter(hazeluser::id.eq(&self.id))).set(self as &User).get_result(connection))
    }

    pub fn generate_apikey<C: Connection<Backend=Pg>>(&mut self, connection: &C) -> BackendResult<Self>
    {
        self.apikey = Some(Uuid::new_v4().simple().to_string());
        self.update(connection)
    }

    pub fn revoke_apikey<C: Connection<Backend=Pg>>(&mut self, connection: &C) -> BackendResult<Self>
    {
        self.apikey = None;
        self.update(connection)
    }

    pub fn get_by_apikey<C: Connection<Backend=Pg>>(connection: &C, apikey: &String) -> BackendResult<Self>
    {
        err!(hazeluser::table.filter(
                hazeluser::apikey.eq(Some(apikey))
            ).first(connection))
    }

    pub fn ensure_admin<C: Connection<Backend=Pg>>(connection: &C, password: String) -> BackendResult<()>
    {
        let username = String::from("admin");
        let fullname = username.clone();
        match try!(hazeluser::table.filter(
                hazeluser::id.eq(&username)
            ).first(connection).optional()) as Option<User>
        {
            Some(mut admin) => {
                admin.password = Some(try!(bcrypt::hash(&*password, bcrypt::DEFAULT_COST)));
                try!(update(hazeluser::table.filter(hazeluser::id.eq(&admin.id))).set(&admin).execute(connection));
                Ok(())
            },
            None => {
                try!(User::new(connection, username, fullname, None, Authentication::Plain(try!(bcrypt::hash(&*password, bcrypt::DEFAULT_COST))), None));
                Ok(())
            }
        }
    }

    pub fn is_admin(&self) -> bool
    {
        self.id == "admin"
    }

    pub fn register<C: Connection<Backend=Pg>>(connection: &C, username: String, fullname: String, mail: String, password: String) -> BackendResult<Self>
    {
        match try!(hazeluser::table.filter(
                hazeluser::id.eq(&username)
            ).first(connection).optional()) as Option<User>
        {
            Some(_) => Err(BackendError::UserAlreadyExists),
            None => {
                if User::ldap_common_name(&username).is_ok() {
                    Err(BackendError::UserAlreadyExists)
                } else {
                    connection.transaction(|| {
                        let mut user = try!(User::new(connection, username, fullname, Some(mail), Authentication::Plain(try!(bcrypt::hash(&*password, bcrypt::DEFAULT_COST))), None));
                        if CONFIG.auth.mail.is_some() {
                            user = try!(user.set_confirmed(connection, false));
                            user = try!(user.send_mail().map(|_| user));
                        } else {
                            user = try!(user.set_confirmed(connection, true));
                        }
                        Ok(user)
                    })
                }
            }
        }
    }

    pub fn update_pass<C: Connection<Backend=Pg>>(&mut self, connection: &C, password: String) -> BackendResult<Self>
    {
        if !self.is_plainauth() {
            return Err(BackendError::PermissionDenied);
        }

        self.password = Some(try!(bcrypt::hash(&*password, bcrypt::DEFAULT_COST)));
        self.update(connection)
    }

    pub fn is_plainauth(&self) -> bool {
        match &*self.provider {
            "Plain" => true,
            _ => false,
        }
    }

    pub fn login<C: Connection<Backend=Pg>>(connection: &C, username: &String, password: &String) -> BackendResult<bool>
    {
        match try!(hazeluser::table.filter(
                hazeluser::id.eq(username)
            ).first(connection).optional()) as Option<User>
        {
            Some(user) => {
                match &*user.provider {
                    "LDAP" => {
                        Ok(User::ldap_login(username, password).is_ok())
                    },
                    "Plain" => {
                        match user.password {
                            Some(stored_hash) => {
                                if try!(bcrypt::verify(&*password, &*stored_hash)) {
                                    Ok(true)
                                } else {
                                    Ok(false)
                                }
                            },
                            None => {
                                Err(LoginError::NoPassHash(username.clone().into()).into())
                            },
                        }
                    },
                    _ => {
                        Err(LoginError::InvalidProvider(username.clone().into()).into())
                    }
                }
            },
            None => {
                match User::ldap_login(username, password) {
                    Ok(full_name) => {
                        try!(User::new(connection, username.clone(), full_name, None, Authentication::LDAP, None));
                        Ok(true)
                    },
                    x @ Err(LDAPError::FilterNotUnique) => err!(x.map(|_| false)),
                    Err(LDAPError::CLDAPError(x)) => { warn!("LDAP Failed: \"{}\"\n Ignoring and proceeding.", x); Ok(false) },
                    Err(x) => {
                        info!("{:?}", x);
                        Ok(false)
                    },
                }
            }
        }
    }

    fn ldap_common_name(username: &String) -> Result<String, LDAPError>
    {
        match CONFIG.auth.ldap {
            None => Err(LDAPError::NotConfigured),
            Some(ref ldap_config) => {
                let mut uri = ldap_config.server_uri.clone();
                uri.push('\0');
                let mut functional_user = ldap_config.login_mask.replace(&ldap_config.login_mask_cn_substitution, &*ldap_config.common_name);
                functional_user.push('\0');
                let mut functional_pass = ldap_config.password.clone();
                functional_pass.push('\0');
                let mut search_scope = ldap_config.scope.clone();
                search_scope.push('\0');
                let mut search_filter = ldap_config.filter.replace(&ldap_config.filter_username_substitution, username);
                search_filter.push('\0');

                let conn = try!(RustLDAP::new(&uri));
                try!(conn.simple_bind(&*functional_user, &functional_pass));

                let entries = try!(conn.ldap_search(&*search_scope, LDAP_SCOPE_SUBTREE, Some(&search_filter), None, false, None, None, ptr::null(), -1));
                match entries.len() {
                    0 => Err(LDAPError::UserNotFound),
                    1 => Ok(entries[0].get("cn").unwrap()[0].clone()),
                    _ => Err(LDAPError::FilterNotUnique),
                }
            }
        }
    }

    fn ldap_login(username: &String, password: &String) -> Result<String, LDAPError>
    {
        match CONFIG.auth.ldap {
            None => Err(LDAPError::NotConfigured),
            Some(ref ldap_config) => {
                let common_name = try!(User::ldap_common_name(username));
                debug!("Ldap Common Name: {}", common_name);
                let mut uri = ldap_config.server_uri.clone();
                uri.push('\0');
                let mut user = ldap_config.login_mask.replace(&ldap_config.login_mask_cn_substitution, &*common_name);
                user.push('\0');
                let mut pass = password.clone();
                pass.push('\0');
                let conn = try!(RustLDAP::new(&*uri));
                err!(conn.simple_bind(&*user, &pass).map(|_| common_name))
            }
        }
    }
}
