.PHONY = check

check:
	cargo fmt
	cargo check
	# https://zhauniarovich.com/post/2021/2021-09-pedantic-clippy/#paranoid-clippy
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-D clippy::nursery \
		-A clippy::option-if-let-else \
		-D clippy::cargo \
		# -D clippy::restriction \
		-D clippy::pedantic
