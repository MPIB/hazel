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

use semver::{ReqParseError, VersionReq, Predicate, PredBuilder, Op, Identifier, WildcardVersion};

use regex::Regex;

use ::utils::error::NugetVersionError;

lazy_static! {
    static ref VERSION_REG_REGEX: Regex = Regex::new(r"^(\[|\()?\s*((\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-(\w+))?)?\s*(,?)\s*((\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-(\w+))?)?\s*(\]|\))?$").unwrap();
}

pub trait NugetToSemver
{
    fn convert(nuget_requirement: &str) -> Result<VersionReq, ReqParseError>;
    fn to_nuget(&self) -> Result<Option<String>, NugetVersionError>;
}

impl NugetToSemver for VersionReq
{
    fn convert(nuget_requirement: &str) -> Result<VersionReq, ReqParseError>
    {
        let caps = match VERSION_REG_REGEX.captures_iter(nuget_requirement).next() {
            None => return Err(ReqParseError::InvalidVersionRequirement),
            Some(x) => x,
        };

        let range_start = caps.get(1);
        let ver1 = caps.get(2);
        let ver1_major = caps.get(3);
        let ver1_minor = caps.get(4);
        let ver1_patch = caps.get(5);
        let ver1_pre = caps.get(6);
        let comma = caps.get(7);
        let ver2 = caps.get(8);
        let ver2_major = caps.get(9);
        let ver2_minor = caps.get(10);
        let ver2_patch = caps.get(11);
        let ver2_pre = caps.get(12);
        let range_stop = caps.get(13);

        let mut builder_min = PredBuilder::new();
        let mut builder_max = PredBuilder::new();

        // ignore some invalid ones, that are not easily parsed by the regex
        if range_start.is_some() != range_stop.is_some() {
            return Err(ReqParseError::InvalidVersionRequirement);
        }
        if comma.is_some() && ver1.is_some() && ver2.is_none() && range_stop.is_some() && range_stop.unwrap().as_str() == "]" {
            return Err(ReqParseError::InvalidVersionRequirement);
        }
        if comma.is_some() && ver1.is_none() && ver2.is_some() && range_start.is_some() && range_start.unwrap().as_str() == "[" {
            return Err(ReqParseError::InvalidVersionRequirement);
        }

        match range_start.map(|x| x.as_str()) {
            None => try!(builder_min.set_op(Op::GtEq)),
            Some("[") => {
                match comma {
                    Some(_) => try!(builder_min.set_op(Op::GtEq)),
                    None => match range_stop.map(|x| x.as_str()) {
                        Some("]") => try!(builder_max.set_op(Op::Ex)),
                        Some(")") => return Err(ReqParseError::InvalidVersionRequirement),
                        _ => unreachable!(),
                    }
                }
            }
            Some("(") => try!(builder_min.set_op(Op::Gt)),
            _ => unreachable!(),
        };
        let pred_min = match ver1 {
            None => None,
            Some(_) => {
                builder_min.major = ver1_major.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_min.minor = ver1_minor.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_min.patch = ver1_patch.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_min.has_pre = ver1_pre.is_some();
                if ver1_pre.is_some() {
                    builder_min.pre.push(match ver1_pre.unwrap().as_str().parse().ok() {
                        Some(n) => Identifier::Numeric(n),
                        None => Identifier::AlphaNumeric(String::from(ver1_pre.unwrap().as_str())),
                    })
                }
                Some(try!(builder_min.build()))
            }
        };

        match range_stop.map(|x| x.as_str()) {
            None | Some("]") => try!(builder_max.set_op(Op::LtEq)),
            Some(")") => try!(builder_max.set_op(Op::Lt)),
            _ => unreachable!(),
        };
        let pred_max = match ver2 {
            None => None,
            Some(_) => {
                builder_max.major = ver2_major.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_max.minor = ver2_minor.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_max.patch = ver2_patch.and_then(|x| x.as_str().parse::<u64>().ok());
                builder_max.has_pre = ver2_pre.is_some();
                if ver2_pre.is_some() {
                    builder_max.pre.push(match ver2_pre.unwrap().as_str().parse().ok() {
                        Some(n) => Identifier::Numeric(n),
                        None => Identifier::AlphaNumeric(String::from(ver2_pre.unwrap().as_str())),
                    })
                }
                Some(try!(builder_max.build()))
            }
        };

        let predicates: Vec<Predicate> = pred_min.iter().chain(pred_max.iter()).cloned().collect();
        Ok(VersionReq::new(&predicates))
    }

    fn to_nuget(&self) -> Result<Option<String>, NugetVersionError>
    {
        let predicates = self.predicates();

        if predicates.len() == 0 {
            Ok(None)
        } else if predicates.len() == 1 {
            let version = predicates[0].version_str();
            match predicates[0].operation() {
                Op::Ex => Ok(Some(format!("[{}]", version))),
                Op::Gt => Ok(Some(format!("({},)", version))),
                Op::GtEq => Ok(Some(version)),
                Op::Lt => Ok(Some(format!("(,{})", version))),
                Op::LtEq => Ok(Some(format!("(,{}]", version))),
                Op::Tilde => Ok(Some(format!("[{},{})", predicates[0].min_version(),
                {
                    let mut next_minor = predicates[0].min_version();
                    next_minor.increment_minor();
                    next_minor
                }))),
                Op::Compatible => Ok(Some(format!("[{},{})", predicates[0].min_version(),
                {
                    let mut next_major = predicates[0].min_version();
                    next_major.increment_major();
                    next_major
                }))),
                Op::Wildcard(wildcard) => {
                    match wildcard {
                        WildcardVersion::Major => Ok(None),
                        WildcardVersion::Minor => Ok(Some(format!("[{},{})", predicates[0].min_version(),
                        {
                            let mut next_major = predicates[0].min_version();
                            next_major.increment_major();
                            next_major
                        }))),
                        WildcardVersion::Patch => Ok(Some(format!("[{},{})", predicates[0].min_version(),
                        {
                            let mut next_minor = predicates[0].min_version();
                            next_minor.increment_minor();
                            next_minor
                        }))),
                    }
                }
            }
        } else if predicates.len() == 2 {
            let version1 = predicates[0].version_str();
            let version2 = predicates[1].version_str();
            Ok(Some(format!("{}, {}",
                match predicates[0].operation() {
                    Op::Gt => format!("({}", version1),
                    Op::GtEq => format!("[{}", version1),
                    _ => return Err(NugetVersionError::InvalidLowerBoundOp),
                },
                match predicates[1].operation() {
                    Op::Lt => format!("{})", version2),
                    Op::LtEq => format!("{}]", version2),
                    _ => return Err(NugetVersionError::InvalidUpperBoundOp),
                }
            )))
        } else {
            return Err(NugetVersionError::MultiPredicate)
        }
    }
}

#[test]
fn solo() {
    use semver::Version;

    let req = VersionReq::convert("1.0").unwrap();
    assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.1.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.4").unwrap()));
    assert!(!req.matches(&Version::parse("3.0.0-alpha1").unwrap()));
}

#[test]
fn pre() {
    use semver::Version;

    let req = VersionReq::convert("1.0.0-alpha1").unwrap();
    assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.1.0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0-prealpha0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0-alpha1").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0-alpha2").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0-beta1").unwrap()));
}

#[test]
fn solo_two() {
    use semver::Version;

    let req = VersionReq::convert("[1.0,)").unwrap();
    assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.1.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.4").unwrap()));
    assert!(!req.matches(&Version::parse("3.0.0-alpha1").unwrap()));
}

#[test]
fn solo_three() {
    use semver::Version;

    let req = VersionReq::convert("(,1.0]").unwrap();
    assert!(req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.1.0").unwrap()));
}

#[test]
fn solo_four() {
    use semver::Version;

    let req = VersionReq::convert("(,1.0)").unwrap();
    assert!(req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.1.0").unwrap()));
}

#[test]
fn minimum() {
    use semver::Version;

    let req = VersionReq::convert("(1.0,)").unwrap();
    assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.1.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.4").unwrap()));
    assert!(!req.matches(&Version::parse("3.0.0-alpha1").unwrap()));
}

#[test]
fn full() {
    use semver::Version;

    let req = VersionReq::convert("(1.0.0,3.0.1]").unwrap();
    assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.1.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.0").unwrap()));
    assert!(req.matches(&Version::parse("2.0.4").unwrap()));
    assert!(!req.matches(&Version::parse("3.0.0-alpha1").unwrap()));
    assert!(req.matches(&Version::parse("3.0.1").unwrap()));
    assert!(!req.matches(&Version::parse("3.0.2").unwrap()));
    assert!(!req.matches(&Version::parse("3.1.0").unwrap()));
    assert!(!req.matches(&Version::parse("4.0.0").unwrap()));
}
