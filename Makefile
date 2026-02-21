.PHONY: check publish-dry-run publish publish-dry-run-dirty publish-dirty

CRATE_NAME := anthropic-sdk-rs
MANIFEST := crates/anthropic-sdk/Cargo.toml
PUBLISH_ENV := env -u http_proxy -u https_proxy -u all_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY CARGO_HTTP_PROXY= GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null

check:
	cargo check -p $(CRATE_NAME)

publish-dry-run:
	$(PUBLISH_ENV) cargo publish --manifest-path $(MANIFEST) --dry-run

publish:
	$(PUBLISH_ENV) cargo publish --manifest-path $(MANIFEST)

publish-dry-run-dirty:
	$(PUBLISH_ENV) cargo publish --manifest-path $(MANIFEST) --dry-run --allow-dirty

publish-dirty:
	$(PUBLISH_ENV) cargo publish --manifest-path $(MANIFEST) --allow-dirty
