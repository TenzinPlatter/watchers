# [1.8.0](https://github.com/TenzinPlatter/watchers/compare/v1.7.1...v1.8.0) (2025-12-12)


### Features

* automatically keep systemd template up to date ([147c36d](https://github.com/TenzinPlatter/watchers/commit/147c36df41cb6e114e1d319e5787a6e49337a602))

## [1.7.1](https://github.com/TenzinPlatter/watchers/compare/v1.7.0...v1.7.1) (2025-12-11)


### Bug Fixes

* add proper handling for deleted files ([99356fc](https://github.com/TenzinPlatter/watchers/commit/99356fcbeb30c59eaedbbfff8df06320cd33bf72))
* correct WantedBy in unit file ([42d06d7](https://github.com/TenzinPlatter/watchers/commit/42d06d7c4caed1191909372c401e597ce5d047e9))

# [1.7.0](https://github.com/TenzinPlatter/watchers/compare/v1.6.1...v1.7.0) (2025-11-13)


### Features

* add logs command ([dcd9072](https://github.com/TenzinPlatter/watchers/commit/dcd90722487e06bf07b842d2f74167b66141cf41))

## [1.6.1](https://github.com/TenzinPlatter/watchers/compare/v1.6.0...v1.6.1) (2025-11-10)


### Bug Fixes

* add version command ([48ae89e](https://github.com/TenzinPlatter/watchers/commit/48ae89eb42e0cfcbc6d5f59a42b46117bba28e61))

# [1.6.0](https://github.com/TenzinPlatter/watchers/compare/v1.5.0...v1.6.0) (2025-11-09)


### Features

* add commit on startup ([ee81793](https://github.com/TenzinPlatter/watchers/commit/ee81793a17337f3934fb461b7e78594ea569756c))

# [1.5.0](https://github.com/TenzinPlatter/watchers/compare/v1.4.0...v1.5.0) (2025-11-03)


### Bug Fixes

* use dynamic executable path in systemd service template ([18a05d1](https://github.com/TenzinPlatter/watchers/commit/18a05d1c79b813cd6b827e2ba67a8310873bc841))


### Features

* add submodule commit and push support ([4c34d2f](https://github.com/TenzinPlatter/watchers/commit/4c34d2f140cc7c3eb672aba43757cdff9f96673b))
* check if file was git ignored when triggering watcher ([17bfef8](https://github.com/TenzinPlatter/watchers/commit/17bfef8d1c7be371c7618b03ef1b18ded6b96f16))
* filter out git internal files and improve git-ignore checking ([7830797](https://github.com/TenzinPlatter/watchers/commit/78307978f6338bd35a7b3ed2ff14f96eebb64f16))

# [1.4.0](https://github.com/TenzinPlatter/watchers/compare/v1.3.0...v1.4.0) (2025-10-06)


### Features

* allow dirty in ci publish ([53fd065](https://github.com/TenzinPlatter/watchers/commit/53fd065d89c1515dc8d10ce85e0f76d5f7cc5726))

# [1.3.0](https://github.com/TenzinPlatter/watchers/compare/v1.2.0...v1.3.0) (2025-10-06)


### Features

* test ci ([7027851](https://github.com/TenzinPlatter/watchers/commit/702785117ead79294feb3aa79d4b7be37723c5e1))

# [1.2.0](https://github.com/TenzinPlatter/watchers/compare/v1.1.0...v1.2.0) (2025-10-06)


### Features

* commit for release ([2720aa3](https://github.com/TenzinPlatter/watchers/commit/2720aa37070a17e99684b75799ced97c99aad084))

# [1.1.0](https://github.com/TenzinPlatter/watchers/compare/v1.0.0...v1.1.0) (2025-10-06)


### Features

* add cli and systemd functionality ([#4](https://github.com/TenzinPlatter/watchers/issues/4)) ([a866d10](https://github.com/TenzinPlatter/watchers/commit/a866d1065b97a7f00bdd44163070f84c5f1e89f4))

# 1.0.0 (2025-09-27)


### Features

* add config module ([ceb85b6](https://github.com/TenzinPlatter/watchers/commit/ceb85b697489ae604d3403d9e3b93aaf1b3dd219))
* async debouncer for commits ([216b83d](https://github.com/TenzinPlatter/watchers/commit/216b83d44a7f0367589eb6cc62d692b5bf0fc385))
* basic file watching ([eae2d70](https://github.com/TenzinPlatter/watchers/commit/eae2d70e9026f395c322991b52b59b81a78bae05))
* now pushes and respects auto commit flag ([c11ef41](https://github.com/TenzinPlatter/watchers/commit/c11ef410569080f895ac0ad0ffad34b04f0e23b7))
* parse a config file and watch directory specified ([621191e](https://github.com/TenzinPlatter/watchers/commit/621191eaa526624acdc08ad90ee1c67a46294753))
* working watcher for git changed files ([66d9993](https://github.com/TenzinPlatter/watchers/commit/66d9993fc3748f031b527a6b902538e174c4a0f2))
