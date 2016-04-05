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

use super::db::PackageVersion;
use semver::Version;

use fs2::FileExt;

use std::fs::{self, File};
use std::path::PathBuf;
use std::io::{self, Read};
use std::sync::Mutex;

pub struct Storage {
    path: Mutex<PathBuf>,
    open_lock: Mutex<()>,
}

impl Storage {
    pub fn new(path: PathBuf) -> Storage {
        fs::create_dir_all(&path).unwrap();
        Storage {
            path: Mutex::new(path),
            open_lock: Mutex::new(()),
        }
    }

    fn get_file_path(&self, package: &PackageVersion) -> PathBuf {
        let mut path = {
            self.path.lock().unwrap().clone()
        };
        path.push(package.id());
        path.set_file_name(package.id().to_string() + "_" + &Version::from(package.version()).to_string() + ".nuget");
        path
    }

    pub fn store<R: Read>(&self, package: &PackageVersion, mut data: R) -> io::Result<()> {
        let path = self.get_file_path(package);

        let mut file = {
            let _ = self.open_lock.lock().unwrap();
            match File::open(path.clone()) {
                Ok(file) => {
                    try!(file.lock_exclusive());
                    try!(file.unlock()); //this is not racey thanks to open_lock.
                }
                Err(_) => {} //file does likely not exist. if the error is different File::create will fail instead
            };

            let truncate = try!(File::create(path));
            try!(truncate.lock_exclusive());
            truncate
            // drop open mutex
        };

        try!(io::copy(&mut data, &mut file));
        Ok(())
    }

    pub fn get(&self, package: &PackageVersion) -> io::Result<File> {
        let path = self.get_file_path(package);

        let file = {
            let _ = self.open_lock.lock().unwrap();
            let file = try!(File::open(path));
            try!(file.lock_exclusive());
            file
            // drop open mutex
        };

        Ok(file)
    }

    pub fn rewrite(&self, package: &PackageVersion, file: File) -> io::Result<File> {
        let path = self.get_file_path(package);

        let new_file = {
            let _ = self.open_lock.lock().unwrap();
            drop(file);
            let new_file = try!(File::create(path));
            try!(new_file.lock_exclusive());
            new_file
            // drop open mutex
        };

        Ok(new_file)
    }

    pub fn delete(&self, package: &PackageVersion) {
        let path = self.get_file_path(package);

        let _ = self.open_lock.lock().unwrap();

        {
            match File::open(path.clone()) {
                Ok(file) => {
                    //wait for exclusivity, while potentially this *could* fail (highly unlikely)
                    //delete is a best-efford succeed method, and we did try
                    let _ = file.lock_exclusive(); //its our
                    let _ = file.unlock();
                    // drop file handle
                },
                Err(x) => debug!("File to remove does not exist? Ignoring ({:?})", x),
            };
        }

        match fs::remove_file(path) {
            Err(x) => info!("Removing file failed, Ignoring ({:?})", x),
            _ => {},
        }

        // drop open mutex
    }
}
