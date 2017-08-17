# <img src="https://github.com/MPIB/hazel/raw/master/icon.png" width="48"> Hazel

## A Chocolatey-compatible multi-platform Package Server written in Rust

Hazel provides a Nuget Feed aimed to be used as source for [chocolatey](https://chocolatey.org/).
It does not aim to be a feature-complete Nuget Feed nor to be 100% standard compliant (although this is desirable),
but rather to provide everything necessary for chocolatey and possibly extend on its functionality.

## Gallery
# <img src="https://github.com/MPIB/hazel/raw/master/resources/screenshots/screenshot1.png" width="800">
# <img src="https://github.com/MPIB/hazel/raw/master/resources/screenshots/screenshot2.png" width="800">

## Building
### Easy way (vagga)
1. Get [vagga](https://github.com/tailhook/vagga)
2. Run `vagga build`

### Hard way
1. Get Rust 1.9.0 or higher
2. Get libssl-dev, libpq-dev, libldap2-dev
3. Run `cargo build --release`
4. (Optional for deb) Get libldap2-dev and pkg-config
5. (Optional for deb) Get cargo-deb (`cargo install --git https://github.com/mmstick/cargo-deb/`)
6. (Optional for deb) Run `cargo deb --no-build`

## Running / Installation

### Testing
1. Get [vagga](https://github.com/tailhook/vagga)
2. Run `vagga run`


### Production
- A Postgres Server accessible by the hazel process (version depends on [diesel](https://diesel.rs) - currently minimum tested is 9.4)
- A file system location to store package
- libraries: openssl, libpq, libldap2
- Build & Run `cargo run` or get Deb Package from [Releases](https://github.com/MPIB/hazel/releases)
- (Optional / Self-Build) Copy the `hazel.toml` from this repo to `/etc/hazel.toml` and the `hazel.service` file to `/etc/systemd/system/`.
- (Optional / Deb) Copy the `hazel.toml` from `/etc/skel/hazel.toml` to `/etc/hazel.toml`
- (Optional) Run hazel through systemd with `systemctl start hazel`

**Note**: Currently hazel.service runs as root by default, you might want to create a new user,
set the appropriate rights on the directories used in the `hazel.toml` and run hazel with limited permissions.

We aim to provide automatically build artifacts in near feature for Windows, OSX and more Linux Distributions.

## Usage
```
$ ./hazel -h
hazel 1.0.0
Victor Brekenfeld <brekenfeld@mpib-berlin.mpg.de>
Chocolatey-compatible Package Server

USAGE:
    hazel [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      Disable console output. Hazel will not make any attempts to open stdout/err
    -V, --version    Prints version information
    -v               Sets the level of verbosity (may be used up to 4 times)

OPTIONS:
    -c, --config <config>      Config file location (e.g. config.toml)
    -d, --db_url <dburl>       Sets the postgres database url (e.g.
                               postgres://postgres:postgr3s@localhost/hazel_feed_v1)
    -l, --logfile <logfile>    Sets a log file path (default: None)
    -p, --port <port>          HTTP port to listen on (default 80)
    -s, --storage <storage>    Sets the storage path (e.g. /var/hazel/storage/) (default current working
                               directory)
```

Every option may also be set via the config file and much more advanced options not available to simple command line parameters. See our [wiki](https://github.com/MPIB/hazel/wiki) for more advanced configurations.

## Installation

Install Rust and run:
```
git clone --branch 1.0.0 http://github.com/mpib-berlin/hazel
cd hazel
cargo build --release --no-default-features --features stable
```

We aim to provide prebuild and tested artifacts in near feature for Windows, OSX and Linux.

If you encounter any errors, feel free to open an issue on our issue tracker.

## Documentation

Additional Documentation and links to Nuget/Chocolatey can be found in our Wiki!

## Contributing

Contributions are highly welcome, just make a Pull Request.
Please keep in mind to make them modular and configurable.

## Copyright & License

This Code is owned by the [Max Planck Institute for Human Development in Berlin, Germany](https://www.mpib-berlin.mpg.de/en).

It is Licensed under the [AGPL-3.0](LICENSE.AGPL3).
It may not be copied, modified, or distributed except according to those terms.

In practice this usually means, you may use the code freely to run your own server, but as soon, as you make modifications you need to disclose them, if your server is publically accessable.
