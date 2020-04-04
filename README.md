# devsecrets
** `devsecrets` makes it easy to access development secrets (e.g.keys, api tokens, etc.) in a safe location apart from your development repository.**

`devsecrets` is a set of tools to keep secrets (e.g.api keys, tokens, etc.) used during
development in a safe location that is easy to access by the project being developed.
This avoids the problem of accidentally comitting such secrets into a git repository, 
which can be a [pain to remove](https://help.github.com/en/github/authenticating-to-github/removing-sensitive-data-from-a-repository).

The tools consist of the `devsecrets` crate and the command-line tool
`cargo-devsecrets` . The former is used by the tool to access data stored in that project's set
of devsecrets, while the latter initializes and helps put files into the set of devsecrets.

# Usage

To initialize the use of `devsecrets` in a crate, first install the `cargo-devsecrets` crate
by running the following:

``` shell
$ cargo install cargo-devsecrets
```

Then run the `init` subcommand of the tool:

``` shell
$ cargo devsecrets init
```

This will write a `devsecrets` ID file ( `.devsecrets_id.txt` ) into your repo, and create a directory that you can
put files that will be accessible to your project via the `devsecrets` crate. To get the path
to that directory, you can run:

``` shell
$ cargo devsecrets path
```

You can also use this command to make it convenient to use in your shell:

``` shell
$ cd $(cargo devsecrets path)
```

If you already have a `devsecrets` ID file in your repository, running `init` will not change the
ID file, but will create the local directory if it does not exist.

# Reading secrets in your project

To use devsecrets from within your project, add the crate to your Cargo.toml using your favorite
method, then add the following macro somewhere in your crate (probably ideally in your
main.rs/lib.rs file):

``` rust
use devsecrets::devsecrets_id;

devsecrets_id!(DEVSECRETS_ID);
```

This will define a `devsecrets::Id` value named `DEVSECRETS_ID` that you can use with the other
parts of the devsecrets API.

To access the devsecrets in your code, you can run:

```
use devsecrets::DevSecrets;

let secrets = DevSecrets::from_id(&DEVSECRETS_ID)?;
```

This will either throw an error if a low-level error occurs, or return an `Option<DevSecrets>`
which will be None if the devsecrets directory has not been initialized.

Once you have a `DevSecrets` value, you can run the various accessors on it to read files from
within that directory.



