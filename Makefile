.DEFAULT_GOAL := help

.PHONY: help install-hooks build lint test guard verify

help:
	@echo "Available targets:"
	@echo "  make install-hooks  安装 Lefthook git hooks"
	@echo "  make build          编译前端与 Rust 后端"
	@echo "  make lint           全量代码检查（CI 级）"
	@echo "  make test           运行单元测试"
	@echo "  make guard          全量门禁：build -> lint -> test -> e2e"
	@echo "  make verify         agent 自校验（等价于 guard）"

install-hooks:
	python tooling/checks.py install-hooks

build:
	python tooling/checks.py build

lint:
	python tooling/checks.py lint --profile ci

test:
	python tooling/checks.py test --profile unit

guard:
	python tooling/checks.py guard

verify:
	python tooling/checks.py verify
