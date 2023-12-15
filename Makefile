.DEFAULT_GOAL := test-sbf

TEST_SBF_CMD = cargo test-sbf -- --nocapture

# Targets and rules
test-sbf:
	$(TEST_SBF_CMD)

t: test-sbf

.PHONY: test-sbf t