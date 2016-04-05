CREATE TABLE HazelUser (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    mail TEXT NULL UNIQUE,
    mail_key TEXT NULL UNIQUE,
    confirmed BOOLEAN NOT NULL,
    provider TEXT NOT NULL,
    password TEXT NULL,
    apikey TEXT NULL,
    PRIMARY KEY(id)
);

CREATE TABLE Package (
    id TEXT NOT NULL,
    project_url TEXT NULL,
    license_url TEXT NULL,
    license_acceptance BOOLEAN NOT NULL DEFAULT 'false',
    project_source_url TEXT NULL,
    package_source_url TEXT NULL,
    docs_url TEXT NULL,
    mailing_list_url TEXT NULL,
    bug_tracker_url TEXT NULL,
    report_abuse_url TEXT NULL,
    maintainer TEXT NOT NULL DEFAULT 'admin',
    PRIMARY KEY(id),
    FOREIGN KEY(maintainer) REFERENCES HazelUser (id)
);

CREATE TABLE PackageVersion (
    id TEXT NOT NULL,
    version TEXT NOT NULL,
    creation_date TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    title TEXT NULL,
    summary TEXT NULL,
    updated TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    description TEXT NULL,
    version_download_count BIGINT NOT NULL DEFAULT '0',
    release_notes TEXT NULL,
    hash TEXT NULL,
    hash_algorithm TEXT NULL,
    size BIGINT NOT NULL,
    icon_url TEXT NULL,
    PRIMARY KEY(id, version),
    FOREIGN KEY(id) REFERENCES Package (id)
);

CREATE TABLE Dependency (
    id TEXT NOT NULL,
    version_req TEXT NOT NULL,
    PRIMARY KEY(id, version_req),
    FOREIGN KEY(id) REFERENCES Package (id)
);

CREATE TABLE PackageVersion_has_Dependency (
    id TEXT NOT NULL,
    dependency_package_id TEXT NOT NULL,
    version TEXT NOT NULL,
    version_req TEXT NOT NULL,
    PRIMARY KEY(id, dependency_package_id, version, version_req),
    FOREIGN KEY(id, version) REFERENCES PackageVersion(id, version),
    FOREIGN KEY(dependency_package_id, version_req) REFERENCES Dependency(id, version_req)
);

CREATE TABLE Author (
    id TEXT NOT NULL,
    PRIMARY KEY(id)
);

CREATE TABLE PackageVersion_has_Author (
    id TEXT NOT NULL,
    version TEXT NOT NULL,
    author_id TEXT NOT NULL,
    PRIMARY KEY(author_id, id, version),
    FOREIGN KEY(author_id) REFERENCES Author (id),
    FOREIGN KEY(id, version) REFERENCES PackageVersion(id, version)
);

CREATE TABLE Tag (
    id TEXT NOT NULL,
    PRIMARY KEY(id)
);

CREATE TABLE Package_has_Tag (
    id TEXT NOT NULL,
    package_id TEXT NOT NULL,
    PRIMARY KEY(id, package_id),
    FOREIGN KEY(id) REFERENCES Tag(id),
    FOREIGN KEY(package_id) REFERENCES Package(id)
);
