SHELL := /bin/bash
help:
	@echo "webInstall - install web dependencies"
	@echo "run - run application"
	@echo "build - build application"


# 定义根路径
ROOT_ENCLOSURE_DIR = src-tauri
ROOT_WEB_DIR = ../n-nacos-web

# 安装 web 依赖
webInstall:
	cd $(ROOT_WEB_DIR) && pnpm i

run:
	$(call webInstall)
	cd $(ROOT_ENCLOSURE_DIR) && pnpm i && pnpm tauri dev

buildDev:
	$(call webInstall)
	cd $(ROOT_ENCLOSURE_DIR) && pnpm i && pnpm tauri build --debug

build:
	$(call webInstall)
	cd $(ROOT_ENCLOSURE_DIR) && pnpm i && pnpm tauri build
    # pnpm tauri build --bundles app
