# pmrmodel

A demo of the core concepts for PMR, written in Rust.

## Build

To build, simply clone this repository, change to that directory, and:

```console
$ cargo build
```

## Usage

To use the demo binary, the database should be built.

```console
$ touch workspace.db
$ source .env
$ sqlx migrate run
```

The `sqlx` utility should be installed from the sqlx-cli crate.
