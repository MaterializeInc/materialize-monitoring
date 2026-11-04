# tflint configuration for every Terraform root under terraform/.
# Run it with `make terraform-lint`, which passes this file by absolute path so
# each recursively-linted directory reads the same rules.
#
# Every rule in the Terraform ruleset is on, and any finding fails the run,
# notices included. Fix a finding rather than disabling its rule. When a
# suppression is genuinely right, put a `# tflint-ignore: <rule>` annotation on
# the line above the finding, with the reason in a comment beside it, so it
# covers that one finding rather than the repo.
#
# The modules use only the helm, kubernetes, and random providers, and tflint
# publishes no ruleset for any of them, so there is no provider plugin here.
# Add one when a module starts declaring a provider that has a ruleset.

# The bundled copy of the ruleset, so its version moves with the tflint release
# pinned in .github/workflows/lint.yaml, and no `tflint --init` download is
# needed.
plugin "terraform" {
  enabled = true
  preset  = "all"
}
