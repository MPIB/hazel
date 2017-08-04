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

mod backend_error {
    use std::borrow::Cow;
    use std::error::Error;
    use std::io;
    use diesel::result as diesel;
    use super::NugetVersionError;
    use super::XmlError;
    use super::LoginError;
    use super::LDAPError;
    use super::MailError;
    use treexml::Error as XmlParseError;
    use semver::{SemVerError, ReqParseError};
    use bcrypt::BcryptError;
    use zip;

    quick_error! {
        #[derive(Debug)]
        pub enum BackendError {
            StorageError(err: io::Error) {
                from()
            }
            ZipError(err: zip::result::ZipError) {
                from()
            }
            DBError(err: diesel::Error) {
                from()
            }
            BlockingDependency(err: Cow<'static, str>) {
                description(&**err)
            }
            XmlError(err: XmlError) {
                from()
                from(e: XmlParseError) -> (e.into())
            }
            InvalidXml(err: Cow<'static, str>) {
                description(&**err)
                from (e: ReqParseError) -> (String::from(e.description()).into())
                from (e: SemVerError) -> (format!("Version is not in semver format: \"{}\"", e.description()).into())
            }
            //This is actually a very critical error, this means data in our DB is actually broken, we need to handle this better (TODO)
            DependencyVersionError(err: NugetVersionError) {
                from()
            }
            CriticalUpdateFailure(err: Cow<'static, str>) {
                description(&**err)
            }
            LoginError(err: LoginError) {
                from()
                from(e: BcryptError) -> (e.into())
                from(e: LDAPError) -> (e.into())
            }
            PermissionDenied {
                display("Permission Denied")
            }
            NotFound {
                display("No entry with the constraints found")
            }
            InvalidProviderForOP {
                display("Operation not allowed for Provider used")
            }
            UserAlreadyExists {
                display("User does already exist")
            }
            MailError(err: MailError) {
                from()
            }
        }
    }
}

mod mail_error {
    use lettre::transport::smtp::error::Error as SmtpError;

    quick_error! {
        #[derive(Debug)]
        pub enum MailError {
            ConfigMissing {
                display("Please specify a mail configuration to be able to send confirmation mails in the Config File")
            }
            UserHasNoMailAddress {
                display("The user is not registered with a mail address, no confirmation mail may be send")
            }
            UnknownAuthenticationMechanism {
                display("The specified authentication mechanism is unknown. Please use either of \"CramMd5\" or \"Plain\"")
            }
            SmtpError(err: SmtpError) {
                from(e: SmtpError) -> (e.into())
            }
        }
    }
}

mod xml_error {
    use std::borrow::Cow;
    use treexml::Error as XmlParseError;

    quick_error! {
        #[derive(Debug)]
        pub enum XmlError {
            XmlParseError(err: XmlParseError) {
                from()
            }
            XmlFindError(err: Cow<'static, str>) {
                description(&**err)
                from (s: &'static str) -> (s.into())
                from (s: String) -> (s.into())
            }
        }
    }
}

mod version_error {

    use std::error::Error;
    use std::fmt;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NugetVersionError {
        MultiPredicate,
        InvalidLowerBoundOp,
        InvalidUpperBoundOp,
    }

    impl Error for NugetVersionError
    {
        fn description(&self) -> &str
        {
            match self {
                &NugetVersionError::MultiPredicate => "More Predicates then 2 are not allowed in Nuget Version Range Specifications",
                &NugetVersionError::InvalidLowerBoundOp => "When using two Predicates, the first must be a lower bound (> or >=)",
                &NugetVersionError::InvalidUpperBoundOp => "When using two Predicates, the second must be an upper bound (< or <=)",
            }
        }
    }

    impl fmt::Display for NugetVersionError
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.description())
        }
    }
}

mod login_error {
    use std::borrow::Cow;
    use bcrypt::BcryptError;
    use super::LDAPError;

    quick_error! {
        #[derive(Debug)]
        pub enum LoginError {
            NoPassHash(username: Cow<'static, str>) {
                display("Broken Password for Plain Provider on \"{}\"", &**username)
            }
            InvalidProvider(username: Cow<'static, str>) {
                display("Broken Password Provider for \"{}\"", &**username)
            }
            Bcrypt(err: BcryptError) {
                from()
            }
            LDAPError(err: LDAPError) {
                from()
            }
        }
    }
}

mod ldap_error {
    use std::borrow::Cow;

    quick_error! {
        #[derive(Debug)]
        pub enum LDAPError {
            NotConfigured {
                display("LDAP not configured")
            }
            UserNotFound {
                display("User not found")
            }
            FilterNotUnique {
                display("Multiple users found, search_filter not unique")
            }
            CLDAPError(err: Cow<'static, str>) {
                display("ldap_internal: {}", &**err)
                from (s: &'static str) -> (s.into())
                from (s: String) -> (s.into())
            }
        }
    }
}

pub use self::backend_error::BackendError;
pub use self::version_error::NugetVersionError;
pub use self::mail_error::MailError;
pub use self::xml_error::XmlError;
pub use self::login_error::LoginError;
pub use self::ldap_error::LDAPError;

pub type BackendResult<T> = Result<T, BackendError>;

macro_rules! err {
    ($error:expr) => (
        match $error {
            Ok(x) => Ok(x),
            Err(x) => return Err(x.into()),
        }
    )
}

macro_rules! err_discard {
    ($error:expr) => (
        match $error {
            Ok(_) => Ok(()),
            Err(x) => return Err(x.into()),
        }
    )
}
