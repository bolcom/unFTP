# Contributing to unFTP

Great projects are never established alone. Therefore we thank you for your interest to help out and offer this guide to
help you along.

unFTP welcomes contribution from everyone in the form of suggestions, bug reports, pull requests, and feedback.

Please reach out here in a GitHub issue if we can do anything to help you contribute.

## Submitting bug reports and feature requests

When reporting a bug or asking for help, please include enough details so that the people helping you can reproduce the behavior you are seeing. For some tips on how to approach this, read about how to produce a [Minimal, Complete, and Verifiable example](https://stackoverflow.com/help/mcve).

When making a feature request, please make it clear what problem you intend to solve with the feature, any ideas for how unFTP could support solving that problem, any possible alternatives, and any disadvantages.

## Checking your code

We encourage you to check that the test suite passes locally and make sure that clippy and rustfmt are happy before submitting a pull request with your changes. If anything does not pass, typically it will be easier to iterate and fix it locally than waiting for the CI servers to run tests for you. Pull requests that do not pass the CI pipeline will not be merged.

##### In the project root

```sh
# Run all tests
cargo test --all-features
cargo clippy --all-features
cargo rustfmt
```
