# How to release?

1. Bump the version in [Cargo.toml](../Cargo.toml) according to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
2. Update [Cargo.lock](../Cargo.lock) by building the project: `cargo build`
3. Ensure [CHANGELOG.md](../CHANGELOG.md) is updated according to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format. [git-cliff](https://github.com/orhun/git-cliff) recommended. (Run `git-cliff > CHANGELOG.md` Then edit Unreleased to next tag and add release date like others)
4. Commit and push the changes.
5. Create a new tag: `git tag -s -a v[x.y.z]` ([signed](https://keyserver.ubuntu.com/pks/lookup?search=0xA89C8C7D22FFF4FB&op=vindex))
6. Push the tag: `git push --tags`
7. Wait for [Continuous Deployment](https://github.com/ErenayDev/checkpoint-ts/actions) workflow to finish.
8. Publish to crates.io: `cargo publish`
9. Update [AUR](https://aur.archlinux.org) package in [PKGBUILDs](https://github.com/ErenayDev/PKGBUILDs) repository:
   - Run `update.sh checkpoint-ts [x.y.z]`
   - Run `aurpublish checkpoint-ts`
