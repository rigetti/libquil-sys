# libquil-sys

This crate provides Rust bindings to the [`quilc`](https://github.com/quil-lang/quilc) C library.

## How to Build

### Install SBCL from source

SBCL must be installed from source to make sure you have the `libsbcl` shared library available.

1. Install `sbcl` from a package manager *for bootstrapping purposes*.
2. Clone the repository: `git clone git://git.code.sf.net/p/sbcl/sbcl`
3. Inside the cloned repo: `sh make.sh && sh make-shared-library.sh`
4. Uninstall the package manager version of `sbcl`.
5. Run `sh install.sh` to install the compiled `sbcl`
6. Copy `src/runtime/libsbcl.so` to `/usr/local/lib/libsbcl.so`

### Set up Lisp workspaces

**Important**: The above `sbcl` folder must **not** be in the workspace folder -- it will cause issues.

1. Create a folder to hold the lisp projects (hereafter: `$LISP_WORKSPACE`)
    - Whenever you see `$LISP_WORKSPACE` below, be sure to use the actual path instead.
2. Clone the following repos into `$LISP_WORKSPACE`
    - <https://github.com/quil-lang/qvm>
    - <https://github.com/quil-lang/magicl>
    - <https://github.com/quil-lang/quilc>
    - <https://github.com/quil-lang/sbcl-librarian>

### Set up Quicklisp

1. Follow the official instructions to [install Quicklisp](https://www.quicklisp.org/beta/#installation).
2. Make sure `$HOME/.sbclrc` contains the following (replace `$LISP_WORKSPACE` with the actual value):

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

### Build `libquil-sys`

From the root of this repository:

1. Edit `build.rs` and replace the hard-coded paths with your local paths:
  - `cargo:rustc-link-search=$LISP_WORKSPACE/quilc/lib`
  - `cargo:rerun-if-changed=$LISP_WORKSPACE/quilc/lib/libquilc.h`
  - `.header("$LISP_WORKSPACE/quilc/lib/libquilc.h")`
  - Required until #2 gets resolved.
2. Run the following commands:
```bash
cp "$LISP_WORKSPACE/quilc/lib/libquilc.dylib" .
cp "$LISP_WORKSPACE/quilc/lib/libquilc.core" .
cargo test
```
