# libquil-sys

This crate provides Rust bindings to the [`quilc`](https://github.com/quil-lang/quilc) C library.

## How to Build

### Install SBCL from source

SBCL must be installed from source to make sure you have the `libsbcl` shared library available.

1. Install `sbcl` from a package manager *for bootstrapping purposes*.
2. Clone the repository: `git clone --branch sbcl-2.2.0 git://git.code.sf.net/p/sbcl/sbcl`
  - Tag `sbcl-2.2.0` is known to work. These instructions have not been tested against a newer version.
3. Inside the cloned repo: `sh make.sh && sh make-shared-library.sh`
4. Uninstall the package manager version of `sbcl`.
5. Run `sh install.sh` to install the compiled `sbcl`
6. Copy `src/runtime/libsbcl.so` to `/usr/local/lib/libsbcl.so`

### Set up Quicklisp

1. Follow the official instructions to [install Quicklisp](https://www.quicklisp.org/beta/#installation).

#### Configure your local projects directory

Quicklisp has a [local projects mechanism](http://blog.quicklisp.org/2018/01/the-quicklisp-local-projects-mechanism.html)
which we'll be using to build `quilc` with it's dependencies. By default, this directory is `$HOME/quicklisp/local-projects`.
If you want to use something different, you need to add to make sure `$HOME/.sbclrc` contains the following
(replace `$LISP_WORKSPACE` with the actual value):

```lisp
;;; The following lines added by ql:add-to-init-file:
#-quicklisp
(let ((quicklisp-init (merge-pathnames "quicklisp/setup.lisp"
                                       (user-homedir-pathname))))
  (when (probe-file quicklisp-init)
    (load quicklisp-init)))

#+quicklisp
(push "$LISP_WORKSPACE" ql:*local-project-directories*)
```

From here on, we'll refer to the local project directory you've chosen to use, whether it's the default or not, as `$LISP_WORKSPACE`.

### Set up Lisp workspaces

**Important**: The above `sbcl` folder must **not** be in the workspace folder -- it will cause issues.

1. Clone the following repos into `$LISP_WORKSPACE`
    - <https://github.com/quil-lang/qvm>
    - <https://github.com/quil-lang/magicl>
    - <https://github.com/quil-lang/quilc>
    - <https://github.com/quil-lang/sbcl-librarian>

### Build `quilc`

**Note**: The build commands in `quilc` assume you are running on MacOS and
[will error on other systems](https://github.com/quil-lang/quilc/issues/861).

Run the following from `$LISP_WORKSPACE`:

```bash
make -C quilc
make -C quilc/lib
```

Optionally, run tests:

```bash
# Optional -- run tests
make -C quilc/tests/c  # Builds executables to manually run
cp quilc/lib/libquilc.core quilc/lib/tests/c/
# MacOS
cp quilc/lib/libquilc.dylib quilc/lib/tests/c/
# *nix
cp quilc/lib/libquilc.so quilc/lib/tests/c/
echo "H 0" | quilc/lib/tests/c/compile-quil
```

### Build and test `libquil-sys`

By default, this library assumes `quilc` is in the default Quicklisp local projects directory (`$HOME/quicklisp/local-projects`).
If you defined a non-default local projects directory for quilc, you need to set `$QUILC_LIBRARY_PATH` to the folder 
where you built the quilc library (the folder containing `libquilc.dylib`). For example,

```bash
export QUILC_LIBRARY_PATH=$LISP_WORKSPACE/quilc/lib
```

Then, from the root of this repository:

```bash
cp "$LISP_WORKSPACE/quilc/lib/libquilc.dylib" .
cp "$LISP_WORKSPACE/quilc/lib/libquilc.core" .
cargo test
```
