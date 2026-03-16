# Changelog

## [0.1.1](https://github.com/subotic/loom/compare/v0.1.0...v0.1.1) (2026-03-16)


### Features

* add two-step org/repo selection in `loom new` and TUI wizard ([c53cc4b](https://github.com/subotic/loom/commit/c53cc4b8d31a5df0648e812789c67908caa8e3cc))
* **agent:** replace repo config warnings with structured confirmations ([2a4c785](https://github.com/subotic/loom/commit/2a4c785782214023a32013afa7c265ea5d7b5fce))
* **cli:** add --groups flag to loom new ([70055d1](https://github.com/subotic/loom/commit/70055d1e3b2fa235650771dcdba753a03dff0ed7))
* **cli:** add --preset flag to loom new and loom refresh ([5abca3a](https://github.com/subotic/loom/commit/5abca3ab808c41fd3316f528df4a2a8c95192c2c))
* **cli:** add loom refresh command ([d659bf8](https://github.com/subotic/loom/commit/d659bf8fd259d7a9463ff51913dc398f052a8565))
* **cli:** add PRESET column to loom list output ([95e3d4e](https://github.com/subotic/loom/commit/95e3d4e19228d8122aaf98d6be6d77759485c3a6))
* **cli:** change org selection from MultiSelect to Select ([f70bf66](https://github.com/subotic/loom/commit/f70bf66c00bcb29ab8e6d942de4aaa2967b0c53f))
* **cli:** make workspace name optional in loom new ([73f0f4f](https://github.com/subotic/loom/commit/73f0f4f5a48af46dac3d794e1195ea80f10dc694)), closes [#11](https://github.com/subotic/loom/issues/11)
* **cli:** show branch in workspace list ([be697fc](https://github.com/subotic/loom/commit/be697fc6b6ab2be5f5b567cab17647cf0db1537b))
* **cli:** wire --verbose and --quiet flags to tracing subscriber ([477f01d](https://github.com/subotic/loom/commit/477f01d1105139d8a5da1df4e470a32229d27174))
* **config:** add interactive security flavor prompt to loom init ([dd39b0f](https://github.com/subotic/loom/commit/dd39b0f5108a9bf445d783af69434b2701fdd3d0))
* **config:** change default workspace folder to ~/workspaces ([d7adb6b](https://github.com/subotic/loom/commit/d7adb6b7d5c0b17776df4884e7213266b5e1d673)), closes [#9](https://github.com/subotic/loom/issues/9)
* implement agent integration (CLAUDE.md and settings.local.json generation) ([392426b](https://github.com/subotic/loom/commit/392426b8567746705ba7f8096caf34e5a373b944))
* implement loom add, remove, and down commands ([8f0e320](https://github.com/subotic/loom/commit/8f0e3208abaab287cd282f7c0d18c1a4daba2a76))
* implement loom exec and loom shell commands ([f301c12](https://github.com/subotic/loom/commit/f301c124daa7408b35918db1300281da69f545d0))
* implement loom init with interactive prompts ([073aac3](https://github.com/subotic/loom/commit/073aac3bc614d3742594261acd37e6003e6dec04))
* implement loom list and loom status commands ([6c138be](https://github.com/subotic/loom/commit/6c138bea20dfb037dd0e6dee3ad23537b037fd52))
* implement loom new with workspace creation and worktree management ([fb0188e](https://github.com/subotic/loom/commit/fb0188e501603c824652293e53a98b3622c7d5e9))
* implement loom save and loom open (cross-machine sync) ([38e0966](https://github.com/subotic/loom/commit/38e09668e394331896737969b7194d2ec3636c17))
* implement TUI with ratatui (workspace list, detail, new wizard) ([41a46fe](https://github.com/subotic/loom/commit/41a46fe0ede574cacc11f213eab5f5e60512264c))
* **update:** add self-update mechanism with loom update command ([78d779a](https://github.com/subotic/loom/commit/78d779ae22e3208459963d22c27330b435856b76))


### Bug Fixes

* **config:** address PR review findings ([7688159](https://github.com/subotic/loom/commit/7688159e246977c6ce40a07e992dab2d8fd48cb3))
* **names:** address review findings for random naming PR ([7985ed0](https://github.com/subotic/loom/commit/7985ed041fe2c8328c1c1af8171249dc6e4cc3e4))
