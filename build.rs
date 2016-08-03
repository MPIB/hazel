// Copyright 2016 Max Planck Institute for Human Development
//
// Licensed under the Apache License, Version 2.0, <LICENSE.APACHE2 or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE.MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(feature = "stable")]
mod inner {
    extern crate syntex;
    extern crate diesel_codegen_syntex as diesel_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();
        diesel_codegen::register(&mut registry);

        let src = Path::new("src/web/backend/db/schema.in.rs");
        let dst = Path::new(&out_dir).join("schema.rs");

        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(not(feature = "stable"))]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
