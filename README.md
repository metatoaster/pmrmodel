# pmrmodel

A demo of the core concepts for PMR, written in Rust.

## Demo

In the cloned directory, run:

```console
$ touch workspace.db
$ source .env 
$ sqlx migrate run
$ cargo build
```

The `sqlx` utility should be installed from the sqlx-cli crate.
