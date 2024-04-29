# syno-photos-util

[![Crates.io
Version](https://img.shields.io/crates/v/syno-photos-util)](https://crates.io/crates/syno-photos-util)

* List folders containing photos in a Synology Photos album
* Copy album contents into a Synology Photos folder

__If you like the project, give it a star ⭐, or consider becoming a__
[![](https://img.shields.io/static/v1?label=Sponsor&message=%E2%9D%A4&logo=GitHub&color=%23fe8e86)](https://github.com/sponsors/caleb9)
:)

- [syno-photos-util](#syno-photos-util)
  - [Why?](#why)
  - [Usage](#usage)
    - [Login to Synology DSM](#login-to-synology-dsm)
    - [List files in an album](#list-files-in-an-album)
    - [Export an album to a folder](#export-an-album-to-a-folder)
    - [Logout](#logout)
  - [Building from source](#building-from-source)
  - [TODO](#todo)
  - [Credits](#credits)

## Why?

This is a console app that queries the Synology Photos API to deduce
locations (folder paths) of photos added to a Synology Photos *album*
or copy photos from an album to a *folder*.

I used it while doing some spring-cleaning of photos on my Synology
NAS (and as a programming exercise in Rust). Maybe someone will find
it handy as well.


## Usage

Display the help message:

```
./syno-photos-util --help
```

(`syno-photos-util.exe` on Windows)

```
Usage: syno-photos-util [OPTIONS] <COMMAND>

Commands:
  login   Sign in to Synology DSM
  list    List file locations (folders) of photos in an album
  export  Export (accessible) album photos to a folder in the user's personal space
  status  Check DSM sign-in status
  logout  Sign out of DSM
  help    Print this message or the help of the given subcommand(s)

Options:
      --timeout <TIMEOUT_SECONDS>
          HTTP request timeout in seconds
          
          Must be greater or equal to 5. When Synology Photos does not respond within the timeout, an
          error is displayed. Try to increase the value for slow connections
          
          [default: 30]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Each command supports the `--help` option for a detailed description.

### Login to Synology DSM

Before you can use any of the other commands, you need to sign in to
DSM:

```
./syno-photos-util login https://your.nas.address/
```

You can provide your DSM user credentials as arguments (see `login
--help`), or you will be asked to type them in. If multi-factor
authentication (MFA) is enabled, you can use the `--remember` option
to not be asked for an OTP code next time you use the `login` command.

> The address value should be the same as the one you use to open DSM
> in your browser. Unless you use non-standard ports (5000 for HTTP
> and 5001 for HTTPS), you can omit the port - otherwise it needs to
> be specified, e.g., `https://your.nas.address:5042`.

> On successful login, the session id is saved into
> <mark>`$HOME/.syno-photos-util`</mark> file (in the user profile
> directory on Windows, e.g., `C:\Users\Alice\.syno-photos-util`),
> similarly to a web browser saving a cookie. Do not share this file
> with anyone, as it gives access to your DSM.

### List files in an album

After signing in successfully, you can list the contents of an album,
printing their file-system paths on your NAS:

```
./syno-photos-util list "My Album"
```

> The album can either be a normal album or a person's name in the
> People albums.

The output may look like this, for example:

```
'/var/services/homes/alice/Photos/PhotoLibrary/2022/11/mountain.jpg'
'/var/services/photo/beach.jpeg'
Error: no access (owned by bob) 'forest.jpg'
```

In this example, My Album contains 3 photos:
* The `mountain.jpg` photo is in one of the signed-in user's (`alice`
  in this case) *Personal Space* folders (located in the user's home
  directory).
* The `beach.jpeg` photo is in *Shared Space* (located in the `photo`
  shared folder).
* In the case of `forest.jpg`, the physical location of the file is
  inaccessible to `alice`. This happens, e.g., when there are other
  NAS users (`bob` in this example) having *provider* access to My
  Album, and they added photos from their Personal Space. Another
  possibility is that My Album is owned by `bob` and shared with
  `alice` - depending on permissions, some or all of the photo
  locations may be inaccessible.

### Export an album to a folder

```
./syno-photos-util export "My Album" "/my folder/my album dump"
```

<mark>Note that currently the target folder needs to already exist in
the user's Personal Space.</mark>

The command schedules a *background task* to copy the photos from an
album to a folder in Personal Space. Photos inaccessible due to
permissions will not be copied. If there are identically named photos
in the target folder already, they will **not** get overwritten. You
can also inspect the task status in Synology Photos web UI.

Because the login session is saved, it is possible to schedule this
command, e.g., with CRON, to export files added to an album
periodically.

### Logout

You may want to logout from DSM when done:

```
./syno-photos-util logout
```

This will remove the session information from
`$HOME/.syno-photos-util`. You may optionally add the `--forget`
option to enforce OTP code verification on the next login (usable only
when MFA is enabled). Alternatively, just deleting the
`$HOME/.syno-photos-util` file has the same effect.

## Building from source

1. [Install Rust](https://www.rust-lang.org/tools/install) if you have
   not already.
2. Install the app from
   [crates.io](https://crates.io/crates/syno-photos-util) (you can use
   the same command to update the app when a new version gets
   published):

   ```bash
   cargo install syno-photos-util
   ```

When building is finished, the binary is then located at
`$HOME/.cargo/bin/syno-photos-util` and should be available on your
`$PATH`.

Alternatively, clone this git repository and build the project with
(in the cloned directory):

```bash
sed -i 's/path = "\(.*\)", version = /version = /' Cargo.toml
cargo build --release
```

`sed` is needed to remove the local paths to dependencies, and install
them from crates.io instead (I'm using a Rust workspace when
developing locally, so the build would fail if that's not set-up on a
fresh clone). The binary is then located at
`target/release/syno-photos-util`.

## TODO

* Add support for "Places" albums
* Add an option for the `export` command to create the target folder

## Credits

* [zeichensatz/SynologyPhotosAPI](https://github.com/zeichensatz/SynologyPhotosAPI)
  contains a description of the Synology Photos API that got me
  started
