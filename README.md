# git-localbranchtidy

A git subcommand that deletes local branches which have already been merged
into the main branch.

## Usage

```
git localbranchtidy [OPTIONS]
```

Run with no options, it detects the main branch and deletes every local
branch whose tip is reachable from it. The current branch and the main
branch itself are always left alone, and `git branch -d` is used so an
unmerged branch is never removed.

### Options

| Option | Description |
| --- | --- |
| `-i`, `--interactive` | Ask before removing each branch. |
| `--main-branch <NAME>` | Use `<NAME>` as the main branch instead of auto-detecting it. |
| `-h`, `--help` | Print help and exit. |
| `-V`, `--version` | Print version and exit. |

### Main branch detection

When `--main-branch` is not given, the main branch is resolved in this order:

1. `origin/HEAD` — the default branch reported by the remote.
2. A local branch named `main`.
3. A local branch named `master`.

If none of those apply, the command exits with an error asking for
`--main-branch`.

## Man page

`man/git-localbranchtidy.1` is installed to `usr/share/man/man1` by the Debian
package. When installing by hand (e.g. `cargo install --path .`, which does not
handle man pages), copy it somewhere on your `MANPATH`:

```
install -Dm644 man/git-localbranchtidy.1 ~/.local/share/man/man1/git-localbranchtidy.1
```

Once it is found by `man`, `git localbranchtidy --help` opens it (git rewrites
`--help` on a subcommand into a man-page lookup); use `-h` for the inline usage
summary.

## Note

Detection relies on the branch tip being an ancestor of the main branch.
Branches that were squash- or rebase-merged do not show up as merged and
are not removed.
